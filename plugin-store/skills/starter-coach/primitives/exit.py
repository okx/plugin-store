"""
Exit condition evaluators — 10 primitives.

G-01: All exits evaluated in parallel every tick. First-to-fire wins.
      On same-tick tie, stop_loss wins (fail-safe).

Each function: evaluate_<type>(ctx, position, params) -> ExitSignal | None
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Any
import time as _time

from primitives.entry import MarketContext


@dataclass
class Position:
    """Open position state."""
    token: str = ""
    entry_price: float = 0.0
    entry_ts: float = 0.0        # unix seconds
    entry_bar_idx: int = 0       # bar index at entry
    size_usd: float = 0.0
    peak_price: float = 0.0      # highest price since entry (for trailing)
    tiers_sold: list[int] = None  # indices of tiered TP levels already sold

    def __post_init__(self):
        if self.tiers_sold is None:
            self.tiers_sold = []
        if self.peak_price == 0:
            self.peak_price = self.entry_price


@dataclass
class ExitSignal:
    """Returned by an exit evaluator when it fires."""
    reason: str           # primitive type name
    sell_pct: float = 100 # % of position to sell (100 = full close)
    priority: int = 0     # lower = higher priority. stop_loss = 0.


# ── Inner exits ──────────────────────────────────────────────────────────────

def evaluate_stop_loss(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-01: Fixed % loss from entry. Always required."""
    pct = params["pct"]
    if pos.entry_price == 0:
        return None
    loss = (pos.entry_price - ctx.current_price) / pos.entry_price * 100
    if loss >= pct:
        return ExitSignal(reason="stop_loss", sell_pct=100, priority=0)
    return None


def evaluate_take_profit(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-02: Fixed % gain from entry."""
    pct = params["pct"]
    if pos.entry_price == 0:
        return None
    gain = (ctx.current_price - pos.entry_price) / pos.entry_price * 100
    if gain >= pct:
        return ExitSignal(reason="take_profit", sell_pct=100, priority=1)
    return None


def evaluate_trailing_stop(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-03: Stop trails pct% below peak price since entry."""
    pct = params["pct"]
    activate_after = params.get("activate_after_pct", 0)
    if pos.entry_price == 0:
        return None
    # Update peak
    if ctx.current_price > pos.peak_price:
        pos.peak_price = ctx.current_price
    # Check activation threshold
    gain_from_entry = (pos.peak_price - pos.entry_price) / pos.entry_price * 100
    if gain_from_entry < activate_after:
        return None
    # Check trailing stop
    drop_from_peak = (pos.peak_price - ctx.current_price) / pos.peak_price * 100
    if drop_from_peak >= pct:
        return ExitSignal(reason="trailing_stop", sell_pct=100, priority=1)
    return None


def evaluate_tiered_take_profit(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-04: Multi-level TP ladder. Sells portions at different profit levels."""
    tiers = params["tiers"]
    if pos.entry_price == 0:
        return None
    gain = (ctx.current_price - pos.entry_price) / pos.entry_price * 100
    for i, tier in enumerate(tiers):
        if i in pos.tiers_sold:
            continue
        if gain >= tier["pct_gain"]:
            pos.tiers_sold.append(i)
            return ExitSignal(
                reason="tiered_take_profit",
                sell_pct=tier["pct_sell"],
                priority=1,
            )
    return None


# ── Other exits (go in exit.other[]) ─────────────────────────────────────────

def evaluate_time_exit(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-05: Exit after max_bars from entry (G-06)."""
    max_bars = params["max_bars"]
    bars_held = len(ctx.bars) - 1 - pos.entry_bar_idx
    if bars_held >= max_bars:
        return ExitSignal(reason="time_exit", sell_pct=100, priority=2)
    return None


def evaluate_indicator_reversal(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-06: Exit when entry indicator flips. Engine checks entry signal == False."""
    # The engine re-evaluates the entry condition. If mirror_entry is true
    # and the entry signal is now False (reversed), exit.
    # This is a meta-evaluator — the engine must wire this up.
    return None  # Engine handles via entry re-evaluation


def evaluate_smart_money_sell(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-07: Exit when >= N smart-money wallets sell. Live-only."""
    min_wallets = params["min_wallets"]
    window_min = params["window_min"]
    now = _time.time()
    cutoff = now - window_min * 60
    sm_sells: set[str] = set()
    for ev in ctx.wallet_events:
        if ev.side == "sell" and ev.token == pos.token and ev.timestamp >= cutoff:
            sm_sells.add(ev.wallet)
    if len(sm_sells) >= min_wallets:
        return ExitSignal(reason="smart_money_sell", sell_pct=100, priority=1)
    return None


def evaluate_dev_dump(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-08: Immediate exit when deployer dumps. Live-only, market-order priority."""
    min_usd = params["min_usd"]
    for ev in ctx.wallet_events:
        if ev.side == "sell" and ev.token == pos.token and ev.usd_amount >= min_usd:
            return ExitSignal(reason="dev_dump", sell_pct=100, priority=0)
    return None


def evaluate_wallet_mirror_sell(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-09: Exit when copied wallet sells. Live-only."""
    target = params["target_wallet"]
    min_pct = params.get("min_pct_sold", 10)
    wallets = [target] if isinstance(target, str) else target
    for ev in ctx.wallet_events:
        if ev.side == "sell" and ev.wallet in wallets and ev.token == pos.token:
            return ExitSignal(reason="wallet_mirror_sell", sell_pct=100, priority=1)
    return None


def evaluate_fast_dump_exit(ctx: MarketContext, pos: Position, params: dict) -> ExitSignal | None:
    """X-10: Emergency exit if price drops X% within N seconds. Live-only."""
    drop_pct = params["drop_pct"]
    window_sec = params["window_sec"]
    if len(ctx.bars) < 2:
        return None
    # Look at recent bars within the window
    now_ts = ctx.bars[-1].ts
    cutoff_ts = now_ts - window_sec
    recent_high = ctx.current_price
    for bar in reversed(ctx.bars):
        if bar.ts < cutoff_ts:
            break
        if bar.high > recent_high:
            recent_high = bar.high
    if recent_high == 0:
        return None
    drop = (recent_high - ctx.current_price) / recent_high * 100
    if drop >= drop_pct:
        return ExitSignal(reason="fast_dump_exit", sell_pct=100, priority=0)
    return None


# ── Dispatcher ───────────────────────────────────────────────────────────────

INNER_EXIT_EVALUATORS: dict[str, Any] = {
    "stop_loss": evaluate_stop_loss,
    "take_profit": evaluate_take_profit,
    "trailing_stop": evaluate_trailing_stop,
    "tiered_take_profit": evaluate_tiered_take_profit,
}

OTHER_EXIT_EVALUATORS: dict[str, Any] = {
    "time_exit": evaluate_time_exit,
    "indicator_reversal": evaluate_indicator_reversal,
    "smart_money_sell": evaluate_smart_money_sell,
    "dev_dump": evaluate_dev_dump,
    "wallet_mirror_sell": evaluate_wallet_mirror_sell,
    "fast_dump_exit": evaluate_fast_dump_exit,
}


def evaluate_all_exits(
    ctx: MarketContext, pos: Position, exit_spec: dict
) -> ExitSignal | None:
    """
    G-01: Evaluate all exits in parallel. Return the first-to-fire.
    On tie, lowest priority value wins (stop_loss = 0).
    """
    signals: list[ExitSignal] = []

    # Inner exits
    for key, evaluator in INNER_EXIT_EVALUATORS.items():
        if key in exit_spec:
            sig = evaluator(ctx, pos, exit_spec[key])
            if sig:
                signals.append(sig)

    # Other exits
    for other in exit_spec.get("other", []):
        etype = other.get("type", "")
        evaluator = OTHER_EXIT_EVALUATORS.get(etype)
        if evaluator:
            params = {k: v for k, v in other.items() if k != "type"}
            sig = evaluator(ctx, pos, params)
            if sig:
                signals.append(sig)

    if not signals:
        return None

    # G-01: On tie, lowest priority wins (stop_loss priority=0 always wins)
    signals.sort(key=lambda s: s.priority)
    return signals[0]
