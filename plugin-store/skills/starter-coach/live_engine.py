"""
Live execution engine — real-time position monitoring + trade execution.

Ties together:
  - onchainos.py (data + swap execution + wallet monitoring)
  - primitives/* (entry/exit/filter/sizing/risk evaluators)
  - paper_gate.py (graduation-gated sizing)
  - harness.py (spec validation)

Usage:
    from live_engine import LiveEngine
    engine = LiveEngine(spec, wallet_address="...")
    engine.start()  # blocking loop
"""
from __future__ import annotations

import json
import time
import logging
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any

from harness import validate_spec
from onchainos import OnchainOS, SwapResult
from primitives.entry import MarketContext, Bar, evaluate_entry
from primitives.exit import Position, ExitSignal, evaluate_all_exits
from primitives.filter import evaluate_filters
from primitives.sizing import compute_size
from primitives.risk import PortfolioState, check_all_overlays
from paper_gate import get_allowed_size_multiplier, record_paper_trade

log = logging.getLogger("live_engine")

# ── Config ────────────────────────────────────────────────────────

TICK_INTERVAL_SEC = 30       # Poll interval for price/candle checks
BALANCE_POLL_SEC = 60        # Poll interval for wallet balance sync
POSITION_STATE_DIR = Path(__file__).parent / ".live_state"

# Common stablecoin addresses per chain (quote tokens for swaps)
QUOTE_TOKENS: dict[str, str] = {
    "solana": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",   # USDC on Solana
    "ethereum": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",   # USDC on Ethereum
    "base": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",       # USDC on Base
    "bsc": "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",        # USDC on BSC
    "arbitrum": "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",    # USDC on Arbitrum
}


# ── Position Tracking ─────────────────────────────────────────────

@dataclass
class LivePosition:
    """A tracked open position."""
    token_address: str = ""
    token_symbol: str = ""
    entry_price: float = 0.0
    entry_ts: float = 0.0
    entry_tx_hash: str = ""
    size_usd: float = 0.0
    token_amount: float = 0.0     # actual tokens held
    peak_price: float = 0.0       # for trailing stop
    current_price: float = 0.0
    unrealized_pnl_usd: float = 0.0
    unrealized_pnl_pct: float = 0.0
    bars_held: int = 0
    status: str = "open"          # open | closing | closed


@dataclass
class EngineState:
    """Persisted engine state across restarts."""
    strategy_name: str = ""
    wallet_address: str = ""
    chain: str = "solana"
    positions: list[dict[str, Any]] = field(default_factory=list)
    closed_trades: list[dict[str, Any]] = field(default_factory=list)
    equity_usd: float = 0.0
    peak_equity_usd: float = 0.0
    trades_today: int = 0
    consecutive_losses: int = 0
    last_trade_day: str = ""
    total_trades: int = 0
    total_pnl_usd: float = 0.0
    started_ts: float = 0.0
    last_tick_ts: float = 0.0


def _state_path(strategy_name: str) -> Path:
    POSITION_STATE_DIR.mkdir(parents=True, exist_ok=True)
    return POSITION_STATE_DIR / f"{strategy_name}.json"


def _load_engine_state(strategy_name: str) -> EngineState:
    path = _state_path(strategy_name)
    if not path.exists():
        return EngineState(strategy_name=strategy_name, started_ts=time.time())
    with open(path) as f:
        data = json.load(f)
    return EngineState(**data)


def _save_engine_state(state: EngineState) -> None:
    state.last_tick_ts = time.time()
    path = _state_path(state.strategy_name)
    with open(path, "w") as f:
        json.dump(asdict(state), f, indent=2)


# ── Live Engine ───────────────────────────────────────────────────

class LiveEngine:
    """
    Real-time execution engine for a validated strategy spec.

    Lifecycle:
      1. validate spec
      2. resolve wallet address
      3. sync initial balances
      4. enter tick loop:
         a. fetch latest candles + price
         b. update position prices + P&L
         c. evaluate exits → fire sells
         d. evaluate entries → fire buys
         e. persist state
    """

    def __init__(
        self,
        spec: dict[str, Any],
        wallet_address: str | None = None,
        initial_equity: float = 0.0,
        paper_mode: bool = False,
    ):
        # Validate spec
        ok, errors, meta = validate_spec(spec)
        if not ok:
            raise ValueError(f"Invalid spec: {'; '.join(errors)}")

        self.spec = spec
        self.meta = meta
        self.paper_mode = paper_mode

        # Extract spec components
        self.strategy_name = spec["meta"]["name"]
        self.chain = spec.get("universe", {}).get("chain", "solana")
        self.symbol = spec["instrument"]["symbol"]
        self.timeframe = spec["instrument"]["timeframe"]
        self.entry_spec = spec["entry"]
        self.exit_spec = spec["exit"]
        self.sizing_spec = spec["sizing"]
        self.filters = spec.get("filters", [])
        self.overlays = spec.get("risk_overlays", [])

        # OnchainOS client
        self.os = OnchainOS(chain=self.chain)

        # Resolve wallet
        self.wallet = wallet_address or self.os.get_wallet_address() or ""
        if not self.wallet and not paper_mode:
            raise ValueError("No wallet address. Log in via `onchainos wallet login`.")

        # Quote token for swaps
        self.quote_token = QUOTE_TOKENS.get(self.chain, "")

        # Graduation multiplier (0.0 = paper, 0.1 = micro, 1.0 = full)
        self.size_multiplier = get_allowed_size_multiplier(self.strategy_name)

        # Load or create engine state
        self.state = _load_engine_state(self.strategy_name)
        self.state.wallet_address = self.wallet
        self.state.chain = self.chain
        if initial_equity > 0:
            self.state.equity_usd = initial_equity
            self.state.peak_equity_usd = initial_equity

        # Rebuild open positions from state
        self.positions: list[LivePosition] = []
        for p_data in self.state.positions:
            self.positions.append(LivePosition(**p_data))

        # Portfolio state for risk overlays
        self.portfolio = PortfolioState(
            trades_today=self.state.trades_today,
            open_positions=len(self.positions),
            open_tokens=[p.token_symbol for p in self.positions],
            equity_usd=self.state.equity_usd,
            peak_equity_usd=self.state.peak_equity_usd,
            consecutive_losses=self.state.consecutive_losses,
            session_start_ts=self.state.started_ts,
        )

        log.info(
            f"LiveEngine initialized: {self.strategy_name} "
            f"chain={self.chain} wallet={self.wallet[:8]}... "
            f"positions={len(self.positions)} equity=${self.state.equity_usd:.2f} "
            f"multiplier={self.size_multiplier}"
        )

    # ── Main Loop ─────────────────────────────────────────────────

    def start(self) -> None:
        """Blocking tick loop. Ctrl-C to stop."""
        log.info(f"Starting live engine for {self.strategy_name}")
        self._sync_balances()

        try:
            while True:
                self._tick()
                time.sleep(TICK_INTERVAL_SEC)
        except KeyboardInterrupt:
            log.info("Engine stopped by user")
        finally:
            self._persist()

    def run_once(self) -> dict[str, Any]:
        """Run a single tick (for testing or cron-based execution)."""
        return self._tick()

    # ── Tick ──────────────────────────────────────────────────────

    def _tick(self) -> dict[str, Any]:
        """Single evaluation cycle."""
        tick_result: dict[str, Any] = {
            "ts": time.time(),
            "exits": [],
            "entries": [],
            "errors": [],
        }

        # Reset daily counters if new day
        today = time.strftime("%Y-%m-%d")
        if self.state.last_trade_day != today:
            self.state.trades_today = 0
            self.state.last_trade_day = today
            self.portfolio.trades_today = 0

        try:
            # Resolve target token(s)
            tokens = self._resolve_tokens()
            if not tokens:
                return tick_result

            for token_addr, token_symbol in tokens:
                # Build market context
                ctx = self._build_context(token_addr, token_symbol)
                if ctx is None:
                    continue

                # 1. Check exits for open positions on this token
                for pos in self.positions:
                    if pos.token_address != token_addr or pos.status != "open":
                        continue
                    pos.current_price = ctx.current_price
                    pos.unrealized_pnl_pct = (
                        (ctx.current_price - pos.entry_price) / pos.entry_price * 100
                        if pos.entry_price > 0 else 0
                    )
                    pos.unrealized_pnl_usd = pos.size_usd * pos.unrealized_pnl_pct / 100
                    if ctx.current_price > pos.peak_price:
                        pos.peak_price = ctx.current_price
                    pos.bars_held += 1

                    position_obj = Position(
                        token=token_symbol,
                        entry_price=pos.entry_price,
                        entry_ts=pos.entry_ts,
                        entry_bar_idx=0,
                        size_usd=pos.size_usd,
                        peak_price=pos.peak_price,
                    )
                    sig = evaluate_all_exits(ctx, position_obj, self.exit_spec)
                    if sig:
                        exit_result = self._execute_exit(pos, sig, ctx)
                        tick_result["exits"].append(exit_result)

                # 2. Check entries if no position on this token
                has_position = any(
                    p.token_address == token_addr and p.status == "open"
                    for p in self.positions
                )
                if not has_position:
                    entry_result = self._try_entry(ctx, token_addr, token_symbol)
                    if entry_result:
                        tick_result["entries"].append(entry_result)

        except Exception as e:
            log.error(f"Tick error: {e}")
            tick_result["errors"].append(str(e))

        self._persist()
        return tick_result

    # ── Token Resolution ──────────────────────────────────────────

    def _resolve_tokens(self) -> list[tuple[str, str]]:
        """
        Resolve which tokens to evaluate this tick.
        Fixed symbol: [(token_address, symbol)]
        Dynamic universe (*): fetch from rankings/wallets.
        """
        if self.symbol != "*":
            # Fixed token — symbol is like "SOL-USDC", extract base
            base = self.symbol.split("-")[0]
            # We need the token address — get from price-info or basic-info
            # For now, use the symbol as-is; the engine caller should provide addresses
            return [(self.symbol, base)]

        # Dynamic universe — fetch from rankings or wallet tracker
        entry_type = self.entry_spec.get("type", "")
        if entry_type == "ranking_entry":
            list_name = self.entry_spec.get("list_name", "trending")
            top_n = self.entry_spec.get("top_n", 20)
            items = self.os.subscribe_ranking(list_name, top_n)
            return [(item.token, item.symbol) for item in items if item.token]
        elif entry_type == "wallet_copy_buy":
            wallets = self.entry_spec.get("target_wallet", [])
            if isinstance(wallets, str):
                wallets = [wallets]
            events = self.os.track_wallets(wallets, trade_type=1)  # buys only
            return list({(e.token, e.token[:8]) for e in events if e.token})
        elif entry_type == "smart_money_buy":
            events = self.os.track_smart_money(trade_type=1)
            return list({(e.token, e.token[:8]) for e in events if e.token})

        return []

    # ── Market Context ────────────────────────────────────────────

    def _build_context(self, token_addr: str, token_symbol: str) -> MarketContext | None:
        """Fetch candles + price and build a MarketContext."""
        candles = self.os.get_candles(token_addr, bar=self.timeframe, limit=100)
        if len(candles) < 10:
            return None

        bars = [
            Bar(
                ts=c["ts"], open=c["open"], high=c["high"],
                low=c["low"], close=c["close"], volume=c["volume"],
            )
            for c in candles
        ]
        current_price = bars[-1].close if bars else 0

        # If current price is 0, try price-info
        if current_price == 0:
            price_data = self.os.get_price_info(token_addr)
            if "error" not in price_data:
                current_price = float(
                    price_data.get("price", price_data.get("lastPrice", 0)) or 0
                )

        if current_price == 0:
            return None

        ctx = MarketContext(
            bars=bars,
            current_price=current_price,
            timeframe=self.timeframe,
            token=token_symbol,
        )

        # Attach on-chain safety data for live_only filters
        # ctx.onchainos is read by primitives/filter.py for safety tag checks
        if self.meta.get("live_only"):
            ctx.onchainos = self.os
            ctx.onchainos_tags = self.os.get_safety_tags(token_addr)

        return ctx

    # ── Entry Execution ───────────────────────────────────────────

    def _try_entry(
        self, ctx: MarketContext, token_addr: str, token_symbol: str
    ) -> dict[str, Any] | None:
        """Evaluate entry conditions and execute buy if triggered."""
        # Check risk overlays
        self.portfolio.equity_usd = self.state.equity_usd
        overlay_ok, blockers = check_all_overlays(self.portfolio, self.overlays)
        if not overlay_ok:
            return None

        # Check filters
        filter_ok, failing = evaluate_filters(ctx, self.filters)
        if not filter_ok:
            return None

        # Check entry trigger
        if not evaluate_entry(ctx, self.entry_spec):
            return None

        # Compute size (with graduation multiplier)
        raw_size = compute_size(self.state.equity_usd, ctx, self.sizing_spec)
        size_usd = raw_size * self.size_multiplier
        if size_usd <= 0:
            return None

        log.info(
            f"ENTRY signal: {token_symbol} @ ${ctx.current_price:.6f} "
            f"size=${size_usd:.2f} (x{self.size_multiplier})"
        )

        # Execute swap
        if self.paper_mode:
            tx_hash = f"paper_{int(time.time())}"
            token_amount = size_usd / ctx.current_price if ctx.current_price > 0 else 0
        else:
            result = self.os.swap_execute(
                from_token=self.quote_token,
                to_token=token_addr,
                readable_amount=str(round(size_usd, 2)),
                wallet=self.wallet,
            )
            if not result.ok:
                log.error(f"Swap failed: {result.error}")
                return {"action": "entry_failed", "error": result.error}
            tx_hash = result.tx_hash
            token_amount = size_usd / ctx.current_price if ctx.current_price > 0 else 0

        # Track position
        pos = LivePosition(
            token_address=token_addr,
            token_symbol=token_symbol,
            entry_price=ctx.current_price,
            entry_ts=time.time(),
            entry_tx_hash=tx_hash,
            size_usd=size_usd,
            token_amount=token_amount,
            peak_price=ctx.current_price,
            current_price=ctx.current_price,
        )
        self.positions.append(pos)
        self.portfolio.open_positions += 1
        self.portfolio.open_tokens.append(token_symbol)
        self.state.trades_today += 1
        self.portfolio.trades_today += 1
        self.state.total_trades += 1

        # Record for paper gate
        if self.paper_mode:
            record_paper_trade(self.strategy_name)

        return {
            "action": "entry",
            "token": token_symbol,
            "price": ctx.current_price,
            "size_usd": size_usd,
            "tx_hash": tx_hash,
            "paper": self.paper_mode,
        }

    # ── Exit Execution ────────────────────────────────────────────

    def _execute_exit(
        self, pos: LivePosition, sig: ExitSignal, ctx: MarketContext
    ) -> dict[str, Any]:
        """Execute a sell based on exit signal."""
        sell_frac = sig.sell_pct / 100
        sell_usd = pos.size_usd * sell_frac
        pnl_pct = (ctx.current_price - pos.entry_price) / pos.entry_price * 100
        pnl_usd = sell_usd * pnl_pct / 100

        log.info(
            f"EXIT signal: {pos.token_symbol} reason={sig.reason} "
            f"sell={sig.sell_pct}% pnl={pnl_pct:+.1f}% (${pnl_usd:+.2f})"
        )

        # Execute sell swap
        tx_hash = ""
        if self.paper_mode:
            tx_hash = f"paper_exit_{int(time.time())}"
        else:
            # Sell the token amount proportional to sell_pct
            sell_amount = pos.token_amount * sell_frac
            if sell_amount > 0:
                result = self.os.swap_execute(
                    from_token=pos.token_address,
                    to_token=self.quote_token,
                    readable_amount=str(sell_amount),
                    wallet=self.wallet,
                )
                if not result.ok:
                    log.error(f"Exit swap failed: {result.error}")
                    return {"action": "exit_failed", "error": result.error}
                tx_hash = result.tx_hash

        # Update state
        self.state.equity_usd += pnl_usd
        self.state.total_pnl_usd += pnl_usd
        if self.state.equity_usd > self.state.peak_equity_usd:
            self.state.peak_equity_usd = self.state.equity_usd
        self.portfolio.equity_usd = self.state.equity_usd
        self.portfolio.peak_equity_usd = self.state.peak_equity_usd

        if pnl_usd < 0:
            self.state.consecutive_losses += 1
            self.portfolio.consecutive_losses += 1
        else:
            self.state.consecutive_losses = 0
            self.portfolio.consecutive_losses = 0

        trade_record = {
            "token": pos.token_symbol,
            "token_address": pos.token_address,
            "entry_price": pos.entry_price,
            "exit_price": ctx.current_price,
            "size_usd": sell_usd,
            "pnl_usd": round(pnl_usd, 4),
            "pnl_pct": round(pnl_pct, 2),
            "reason": sig.reason,
            "entry_ts": pos.entry_ts,
            "exit_ts": time.time(),
            "tx_hash": tx_hash,
            "paper": self.paper_mode,
        }

        if sig.sell_pct >= 100:
            pos.status = "closed"
            self.portfolio.open_positions -= 1
            if pos.token_symbol in self.portfolio.open_tokens:
                self.portfolio.open_tokens.remove(pos.token_symbol)
            self.state.closed_trades.append(trade_record)
        else:
            pos.size_usd *= (1 - sell_frac)
            pos.token_amount *= (1 - sell_frac)

        return {"action": "exit", **trade_record}

    # ── Balance Sync ──────────────────────────────────────────────

    def _sync_balances(self) -> None:
        """Sync wallet balances with on-chain state."""
        if self.paper_mode:
            return

        balances = self.os.get_all_balances(force=True)
        if not balances:
            log.warning("Could not fetch wallet balances")
            return

        # Calculate total equity from balances
        total_usd = 0.0
        for bal in balances:
            if isinstance(bal, dict):
                usd_val = float(bal.get("balanceUsd", bal.get("usd_value", 0)) or 0)
                total_usd += usd_val

        if total_usd > 0:
            self.state.equity_usd = total_usd
            if total_usd > self.state.peak_equity_usd:
                self.state.peak_equity_usd = total_usd
            log.info(f"Wallet equity synced: ${total_usd:.2f}")

    # ── Position Queries ──────────────────────────────────────────

    def get_open_positions(self) -> list[dict[str, Any]]:
        """Return all open positions with current P&L."""
        result = []
        for pos in self.positions:
            if pos.status != "open":
                continue
            result.append({
                "token": pos.token_symbol,
                "token_address": pos.token_address,
                "entry_price": pos.entry_price,
                "current_price": pos.current_price,
                "size_usd": pos.size_usd,
                "unrealized_pnl_usd": pos.unrealized_pnl_usd,
                "unrealized_pnl_pct": pos.unrealized_pnl_pct,
                "bars_held": pos.bars_held,
                "entry_ts": pos.entry_ts,
            })
        return result

    def get_portfolio_summary(self) -> dict[str, Any]:
        """Return portfolio summary for display."""
        open_pos = self.get_open_positions()
        total_unrealized = sum(p["unrealized_pnl_usd"] for p in open_pos)
        return {
            "strategy": self.strategy_name,
            "chain": self.chain,
            "wallet": self.wallet,
            "equity_usd": round(self.state.equity_usd, 2),
            "peak_equity_usd": round(self.state.peak_equity_usd, 2),
            "total_pnl_usd": round(self.state.total_pnl_usd, 2),
            "unrealized_pnl_usd": round(total_unrealized, 2),
            "open_positions": len(open_pos),
            "total_trades": self.state.total_trades,
            "trades_today": self.state.trades_today,
            "consecutive_losses": self.state.consecutive_losses,
            "size_multiplier": self.size_multiplier,
            "paper_mode": self.paper_mode,
            "positions": open_pos,
        }

    def get_trade_history(self) -> list[dict[str, Any]]:
        """Return closed trade history."""
        return list(self.state.closed_trades)

    # ── Persistence ───────────────────────────────────────────────

    def _persist(self) -> None:
        """Save engine state to disk."""
        # Serialize open positions
        self.state.positions = [
            asdict(p) for p in self.positions if p.status == "open"
        ]
        _save_engine_state(self.state)
