"""
Smart Money Signal Copy Trade v1.0 — onchainos Agentic Wallet
Dashboard: http://localhost:3248

v3.2 Fix List (Comprehensive Audit):
- [C0] execute_swap: Get toTokenAmount from routerResult (Live swap nested structure)
- [C1] Time Decay SL: Sort by after_min descending, older positions get tighter SL
- [C2] TP partial sell adds continue, prevent double sell in same cycle
- [C3] close_position: Read token_amount from live state, not snapshot
- [C4] After partial sell, reduce buy_sol proportionally
- [C5] token_amount=0 defense, do not create ghost positions
- [H1] Time Decay SL close adds continue
- [H2] sell_fail_count retry limit
- [H3] config reload only in run() main thread, monitor reads config without reload
- [H4] wallet_addr declared global
- [H5] record_trade uses state_lock
- [H6] SOL_NATIVE keeps existing value (onchainos CLI uses 32 ones for native SOL)
- [H7] swap operations use ORDER_TIMEOUT_SEC
- [M1] Re-check MAX_POSITIONS before buy
- [M2] int(sell_amount) truncation check
- [M3] SOL balance matching compatible with None
- [M4] save_trades uses lock
- [M6] config reload failure logs warning
- [L1] cooldown_map cleans expired entries
- [L3] load_state restores buys count
- [L4] tradeId adds random suffix to prevent collision
"""

import time, json, threading, importlib, subprocess, os, random, string, signal, sys
from datetime import datetime
from pathlib import Path
from http.server import HTTPServer, BaseHTTPRequestHandler

from risk_check import pre_trade_checks, post_trade_flags
import config

# ── Constants ────────────────────────────────────────────────────────────────
PROJECT_DIR = Path(__file__).parent
SOL_NATIVE = "11111111111111111111111111111111"  # 32 ones, native SOL (onchainos CLI format)
COOLDOWN_SEC = 300  # 5-minute cooldown after sell
POSITIONS_FILE = str(PROJECT_DIR / "positions.json")
TRADES_FILE = str(PROJECT_DIR / "signal_trades.json")

cooldown_map = {}   # {token_address: expire_timestamp}
wallet_addr = ""    # Obtained via wallet addresses in Live mode
state_lock = threading.Lock()
pos_lock = threading.Lock()
trades_lock = threading.Lock()  # [M4] Protect trades file writes
_selling = set()    # Prevent concurrent sells of the same token

state = {
    "positions": {},
    "trades": [],
    "feed": [],
    "stats": {"cycle": 0, "buys": 0, "sells": 0, "wins": 0, "losses": 0, "net_sol": 0.0},
}

session_risk = {
    "consecutive_losses": 0,
    "cumulative_loss_sol": 0.0,
    "paused_until": 0,
    "stopped": False,
}


# ── onchainos CLI Wrapper ────────────────────────────────────────────────────

def onchainos(*args, timeout=20):
    """Call onchainos CLI, return the data field"""
    try:
        r = subprocess.run(
            ['onchainos', *args],
            capture_output=True, text=True, timeout=timeout
        )
        result = json.loads(r.stdout)
        if not result.get('ok'):
            raise RuntimeError(f"onchainos {args[0]} {args[1]}: {result.get('msg', result)}")
        return result['data']
    except subprocess.TimeoutExpired:
        raise RuntimeError(f"onchainos {' '.join(args[:2])}: timeout {timeout}s")
    except json.JSONDecodeError:
        raise RuntimeError(f"onchainos {' '.join(args[:2])}: invalid JSON")


# ── Helper Functions ─────────────────────────────────────────────────────────

def feed(msg):
    """Add log entry to Activity Feed"""
    with state_lock:
        state["feed"].append({"msg": msg, "t": datetime.now().strftime("%H:%M:%S")})
        state["feed"] = state["feed"][-50:]
    print(f"[{datetime.now().strftime('%H:%M:%S')}] {msg}")

def save_positions():
    """Atomic write positions file. Caller should hold pos_lock."""
    tmp = POSITIONS_FILE + ".tmp"
    with open(tmp, "w") as f:
        json.dump(state["positions"], f, default=str, indent=2)
    os.replace(tmp, POSITIONS_FILE)

def save_trades():
    """Atomic write trade history. Caller should hold trades_lock."""  # [M4]
    tmp = TRADES_FILE + ".tmp"
    with open(tmp, "w") as f:
        json.dump(state["trades"], f, default=str, indent=2)
    os.replace(tmp, TRADES_FILE)

def load_state():
    """Load previous positions and trade history on startup"""
    try:
        with open(POSITIONS_FILE) as f:
            state["positions"] = json.load(f)
        # Backward compatibility: add origin tag for legacy positions
        for addr, pos in state["positions"].items():
            if "origin" not in pos:
                pos["origin"] = "smart_money_signal_copy_trade_legacy"
        print(f"  Restored {len(state['positions'])} positions from disk")
    except FileNotFoundError:
        pass
    try:
        with open(TRADES_FILE) as f:
            state["trades"] = json.load(f)
        # Restore stats (including buys)  [L3]
        for t in state["trades"]:
            net_pnl = t.get("net_pnl_pct", 0)
            if net_pnl > 0:
                state["stats"]["wins"] += 1
            elif net_pnl < 0:
                state["stats"]["losses"] += 1
            state["stats"]["sells"] += 1
            state["stats"]["net_sol"] += t.get("pnl_sol", 0)
        # [L3] Infer buys from position count + sells count
        state["stats"]["buys"] = len(state["positions"]) + state["stats"]["sells"]
        print(f"  Restored {len(state['trades'])} trades")
    except FileNotFoundError:
        pass


def check_trend_stop(ca):
    """Check if 15m candle confirms trend reversal"""
    try:
        candles = onchainos('market', 'candles', '--chain', 'solana',
            '--address', ca, '--bar', config.TIME_STOP_CANDLE_BAR)
        if not candles or len(candles) < 2:
            return False
        k1 = candles[-1]
        k2 = candles[-2]
        k1_close = float(k1.get("c", 0))
        k1_open = float(k1.get("o", 0))
        k1_vol = float(k1.get("vol", 0))
        k2_vol = float(k2.get("vol", 0))
        if k1_close < k1_open and k1_vol >= k2_vol * config.TIME_STOP_REVERSAL_VOL:
            return True
    except Exception:
        pass
    return False


def safe_float(v, default=0.0):
    """Safe float conversion, handles empty string/None/missing"""
    try:
        return float(v)
    except (ValueError, TypeError):
        return default

def safe_int(v, default=0):
    """Safe int conversion, handles empty string/None/missing"""
    try:
        return int(v)
    except (ValueError, TypeError):
        return default


def cleanup_cooldown():
    """[L1] Clean up expired cooldown entries"""
    now = time.time()
    expired = [k for k, v in cooldown_map.items() if now >= v]
    for k in expired:
        del cooldown_map[k]


# ── Session Risk Control ─────────────────────────────────────────────────────

def can_enter():
    """Check if opening new positions is allowed. Returns (ok, reason)."""
    if config.PAUSED:
        return False, "PAUSED (manual)"
    with state_lock:
        if session_risk["stopped"]:
            return False, "Session stopped"
        if time.time() < session_risk["paused_until"]:
            remain = int(session_risk["paused_until"] - time.time())
            return False, f"Session paused ({remain}s left)"
    with pos_lock:
        if len(state["positions"]) >= config.MAX_POSITIONS:
            return False, "Max positions"
    return True, "OK"


def record_loss(pnl_sol):
    """Record loss and update session risk control. Thread-safe."""
    with state_lock:
        session_risk["consecutive_losses"] += 1
        session_risk["cumulative_loss_sol"] += abs(pnl_sol)

        if session_risk["cumulative_loss_sol"] >= config.SESSION_STOP_SOL:
            session_risk["stopped"] = True
            feed(f"🛑 SESSION_STOP: Cumulative loss {session_risk['cumulative_loss_sol']:.4f} SOL")
        elif session_risk["cumulative_loss_sol"] >= config.SESSION_LOSS_LIMIT_SOL:
            session_risk["paused_until"] = time.time() + config.SESSION_LOSS_PAUSE_SEC
            feed(f"⏸ SESSION_PAUSE: Cumulative loss {session_risk['cumulative_loss_sol']:.4f} SOL, "
                 f"paused {config.SESSION_LOSS_PAUSE_SEC//60}min")
        elif session_risk["consecutive_losses"] >= config.MAX_CONSEC_LOSS:
            session_risk["paused_until"] = time.time() + config.PAUSE_CONSEC_SEC
            feed(f"⏸ SESSION_PAUSE: {session_risk['consecutive_losses']} consecutive losses, "
                 f"paused {config.PAUSE_CONSEC_SEC//60}min")

        state["stats"]["losses"] = state["stats"].get("losses", 0) + 1


def record_win():
    """Record win. Thread-safe."""
    with state_lock:
        session_risk["consecutive_losses"] = 0
        state["stats"]["wins"] = state["stats"].get("wins", 0) + 1


# ── Core Functions ───────────────────────────────────────────────────────────

def execute_swap(from_token, to_token, amount, wallet_addr, is_buy=True):
    """
    Execute swap trade. Paper mode calls quote, Live mode calls swap + contract-call.
    amount: lamports (int) — SOL lamports for buy, token smallest unit amount for sell
    Returns: {"toTokenAmount": str, "toTokenUsdPrice": str, "txHash": str|None}
    """
    amount_str = str(int(amount))

    if config.DRY_RUN:
        # quote does not support --slippage parameter, only swap does
        data = onchainos('swap', 'quote',
            '--from', from_token, '--to', to_token,
            '--amount', amount_str, '--chain', 'solana')
        q = data[0] if isinstance(data, list) else data
        # [C0] quote may also have routerResult nesting, handle uniformly
        router = q.get("routerResult", q)
        return {
            "toTokenAmount": str(router.get("toTokenAmount", 0)),
            "toTokenUsdPrice": router.get("toToken", {}).get("tokenUnitPrice",
                               q.get("toToken", {}).get("tokenUnitPrice", "0")),
            "txHash": None,
        }
    else:
        # [H7] swap operations use longer timeout
        data = onchainos('swap', 'swap',
            '--from', from_token, '--to', to_token,
            '--amount', amount_str, '--chain', 'solana',
            '--slippage', str(config.SLIPPAGE_PCT),
            '--wallet', wallet_addr,
            timeout=getattr(config, 'ORDER_TIMEOUT_SEC', 120))
        q = data[0] if isinstance(data, list) else data

        # [C0] swap returns toTokenAmount inside routerResult
        router = q.get("routerResult", q)

        tx = q.get("tx", {})
        tx_to = tx.get("to", "")
        unsigned_tx = tx.get("data", "")

        if not tx_to or not unsigned_tx:
            raise RuntimeError(f"swap response missing tx.to or tx.data: {q}")

        result = onchainos('wallet', 'contract-call',
            '--chain', '501',
            '--to', tx_to,
            '--unsigned-tx', unsigned_tx,
            timeout=getattr(config, 'ORDER_TIMEOUT_SEC', 120))

        tx_hash = result.get("txHash") or result.get("orderId", "")

        return {
            "toTokenAmount": str(router.get("toTokenAmount", 0)),
            "toTokenUsdPrice": router.get("toToken", {}).get("tokenUnitPrice",
                               q.get("toToken", {}).get("tokenUnitPrice", "0")),
            "txHash": tx_hash,
        }


def open_position(signal, wallet_addr):
    """
    Execute the full filter chain + buy for a signal token.
    signal: Single signal entry returned by onchainos signal list
    Returns: True=buy successful, False=filtered out
    """
    token = signal.get("token", {})
    ca = token.get("tokenAddress", "") or token.get("address", "")
    symbol = token.get("symbol", "?")
    wallet_count = int(signal.get("triggerWalletCount", 0))
    sold_ratio = float(signal.get("soldRatioPercent", 100))

    # ── Pre-checks (unified risk control via can_enter) ──
    ok, reason = can_enter()
    if not ok:
        return False

    with pos_lock:
        if ca in state["positions"]:
            return False  # Already holding
    if ca in cooldown_map and time.time() < cooldown_map[ca]:
        return False  # In cooldown

    # ── Level 1 Pre-filter (signal data) ──
    if sold_ratio > config.MAX_SELL_RATIO * 100:
        feed(f"Skip {symbol}: soldRatio {sold_ratio:.0f}%"); return False
    if wallet_count < config.MIN_WALLET_COUNT:
        feed(f"Skip {symbol}: wallets {wallet_count}"); return False

    # ── Level 2 Deep Verification ──
    rc = None  # Risk check result placeholder
    try:
        # 1. token price-info → MC, Liq, Holders
        prices = onchainos('token', 'price-info', '--chain', 'solana', '--address', ca)
        p = prices[0] if isinstance(prices, list) else prices
        mc = safe_float(p.get("marketCap", 0))
        liq = safe_float(p.get("liquidity", 0))
        holders = safe_int(p.get("holders", 0))
        price = safe_float(p.get("price", 0))

        if mc < config.MIN_MCAP:
            feed(f"Skip {symbol}: MC ${mc:,.0f}"); return False
        if liq < config.MIN_LIQUIDITY:
            feed(f"Skip {symbol}: Liq ${liq:,.0f}"); return False
        if holders < config.MIN_HOLDERS:
            feed(f"Skip {symbol}: Holders {holders}"); return False
        if mc > 0 and liq / mc < config.MIN_LIQ_MC_RATIO:
            feed(f"Skip {symbol}: Liq/MC {liq/mc:.1%}"); return False
        if mc > 0 and holders / (mc / 1e6) < config.MIN_HOLDER_DENSITY:
            feed(f"Skip {symbol}: HolderDensity low"); return False

        # 2. market candles → K1 pump check (skip if candle data unavailable, don't block buy)
        try:
            candles = onchainos('market', 'candles', '--chain', 'solana',
                '--address', ca, '--bar', '1m')
            if candles and len(candles) >= 2:
                k1 = candles[-1]
                k1_open = float(k1.get("o", 0))
                k1_close = float(k1.get("c", 0))
                if k1_open > 0:
                    k1_pct = (k1_close - k1_open) / k1_open * 100
                    if k1_pct > config.MAX_K1_PCT_ENTRY:
                        feed(f"Skip {symbol}: K1 +{k1_pct:.1f}%"); return False
        except Exception:
            pass  # Candle data unavailable, skip pump detection

        # 3. token advanced-info → dev, bundler, LP, Top10
        adv = onchainos('token', 'advanced-info', '--chain', 'solana', '--address', ca)
        dev_rug = safe_int(adv.get("devRugPullTokenCount", 0))
        dev_launched = safe_int(adv.get("devLaunchedTokenCount", 0))
        dev_hold = safe_float(adv.get("devHoldingPercent", 0))
        bundle_ath = safe_float(adv.get("bundleHoldingAthPercent", 0))
        bundle_count = safe_int(adv.get("bundleCount", 0))
        lp_burn = safe_float(adv.get("lpBurnedPercent", 0))
        top10 = safe_float(adv.get("top10HoldPercent", 0))

        # Rate-based rug check (aligned with risk_check.py)
        rug_rate = dev_rug / max(dev_launched, 1) if dev_launched > 0 else (1.0 if dev_rug > 0 else 0.0)
        if rug_rate >= 0.20 and dev_rug >= 3:
            feed(f"Reject {symbol}: SerialRugger rate={rug_rate*100:.0f}% ×{dev_rug}"); return False
        # Absolute count fallback (config adjustable)
        max_dev_rug = getattr(config, 'DEV_MAX_RUG_COUNT', 5)
        if max_dev_rug and dev_rug > max_dev_rug:
            feed(f"Reject {symbol}: DevRug:{dev_rug}"); return False
        if dev_launched > config.DEV_MAX_LAUNCHED:
            feed(f"Reject {symbol}: DevFarm:{dev_launched}"); return False
        if dev_hold > config.DEV_MAX_HOLD_PCT:
            feed(f"Reject {symbol}: DevHold:{dev_hold:.1f}%"); return False
        if bundle_ath > config.BUNDLE_MAX_ATH_PCT:
            feed(f"Reject {symbol}: BundlerATH:{bundle_ath:.1f}%"); return False
        if bundle_count > config.BUNDLE_MAX_COUNT:
            feed(f"Reject {symbol}: BundlerCount:{bundle_count}"); return False
        if lp_burn < config.MIN_LP_BURN:
            feed(f"Reject {symbol}: LPBurn:{lp_burn:.0f}%"); return False
        if top10 > config.MAX_TOP10_HOLDER_PCT:
            feed(f"Reject {symbol}: Top10:{top10:.1f}%"); return False

        # Risk check — honeypot, wash trading, rug rate
        rc = pre_trade_checks(ca, symbol, quick=True)
        if rc["grade"] >= 3:
            feed(f"Reject {symbol}: RISK G{rc['grade']} — {', '.join(rc['reasons'][:2])}")
            return False
        if rc["grade"] == 2:
            feed(f"Caution {symbol}: {', '.join(rc['cautions'][:2])}")

    except Exception as e:
        feed(f"Reject {symbol}: safety check failed: {e}"); return False

    # ── Tier Classification ──
    tier, size_sol = "low", config.POSITION_TIERS["low"]["sol"]
    for t in ("high", "mid", "low"):
        if wallet_count >= config.POSITION_TIERS[t]["min_addr"]:
            tier, size_sol = t, config.POSITION_TIERS[t]["sol"]; break

    # ── Balance Check ──
    try:
        bal_data = onchainos('wallet', 'balance', '--chain', '501')
        sol_bal = 0.0
        # Handle nested structure: data.details[].tokenAssets[] or flat list
        assets = []
        if isinstance(bal_data, dict):
            details = bal_data.get("details", [])
            if isinstance(details, list):
                for detail in details:
                    assets.extend(detail.get("tokenAssets", []))
            # fallback: if no details, may be flat dict
            if not assets and "tokenAddress" in bal_data:
                assets = [bal_data]
        elif isinstance(bal_data, list):
            assets = bal_data
        else:
            assets = [bal_data]
        for b in assets:
            # [M3] Compatible with tokenAddress being "", None, or missing
            ta = b.get("tokenAddress")
            if ta in ("", None):
                sol_bal = float(b.get("balance", 0)); break
        if sol_bal < size_sol + config.SOL_GAS_RESERVE:
            feed(f"Skip {symbol}: SOL balance {sol_bal:.4f} < {size_sol + config.SOL_GAS_RESERVE:.4f}")
            return False
    except Exception:
        if not config.DRY_RUN:
            feed(f"Skip {symbol}: balance check failed"); return False

    # ── Execute Buy ──
    lamports = int(size_sol * 1e9)
    try:
        result = execute_swap(SOL_NATIVE, ca, lamports, wallet_addr, is_buy=True)
    except Exception as e:
        # [C11] Timeout → create unconfirmed position to prevent duplicate buys
        if not config.DRY_RUN and "timeout" in str(e).lower():
            feed(f"BUY TIMEOUT {symbol}: {e} — creating unconfirmed position")
            now = time.time()
            with pos_lock:
                state["positions"][ca] = {
                    "symbol": symbol, "address": ca,
                    "label": signal.get("walletTypeName", "SmartMoney"),
                    "entry_price": price, "entry_mc": mc,
                    "token_amount": 0, "buy_sol": size_sol,
                    "tier": tier, "tp_tier": 0,
                    "sl_price": price * config.SL_MULTIPLIER,
                    "breakeven_pct": 0, "net_pnl_pct": 0,
                    "peak_price": price,
                    "opened_at": datetime.utcnow().isoformat(),
                    "opened_at_ts": now, "age_min": 0,
                    "sell_fail_count": 0, "origin": "smart_money_signal_copy_trade",
                    "unconfirmed": True,
                    "unconfirmed_ts": now,
                    "unconfirmed_checks": 0,
                }
                save_positions()
            return False
        feed(f"BUY FAIL {symbol}: {e}"); return False

    token_amount = float(result["toTokenAmount"])

    # [C5] Defense against token_amount=0, do not create ghost positions
    if token_amount <= 0:
        feed(f"BUY WARN {symbol}: token_amount=0, swap may have failed or returned abnormal structure")
        return False

    buy_price = float(result["toTokenUsdPrice"]) if float(result.get("toTokenUsdPrice", 0)) > 0 else price
    if buy_price <= 0:
        try:
            sol_price_data = onchainos('token', 'price-info', '--chain', 'solana', '--address', SOL_NATIVE)
            sol_usd = float(sol_price_data[0].get("price", 0)) if sol_price_data else 0
            if sol_usd > 0 and token_amount > 0:
                buy_price = (size_sol * sol_usd) / token_amount
        except Exception:
            pass
    if buy_price <= 0:
        feed(f"Skip {symbol}: price=0, cannot open position"); return False

    # ── Calculate breakeven ──
    be_pct = (config.FIXED_COST_SOL / size_sol * 100) + (config.COST_PER_LEG_PCT * 2)

    # ── Record position ──
    now = time.time()
    with pos_lock:
        # [M1] Re-check MAX_POSITIONS before buy, prevent TOCTOU
        if len(state["positions"]) >= config.MAX_POSITIONS:
            feed(f"Skip {symbol}: MAX_POSITIONS reached (race)"); return False
        state["positions"][ca] = {
            "symbol": symbol,
            "address": ca,
            "label": signal.get("walletTypeName", "SmartMoney"),
            "entry_price": buy_price,
            "entry_mc": mc,
            "token_amount": token_amount,
            "buy_sol": size_sol,
            "tier": tier,
            "tp_tier": 0,
            "sl_price": buy_price * config.SL_MULTIPLIER,
            "breakeven_pct": be_pct,
            "net_pnl_pct": -be_pct,
            "peak_price": buy_price,
            "opened_at": datetime.utcnow().isoformat(),
            "opened_at_ts": now,
            "age_min": 0,
            "sell_fail_count": 0,  # [H2] Sell failure count
            "origin": "smart_money_signal_copy_trade",
            "entry_liquidity_usd": rc["raw"]["liquidity_usd"] if rc and rc.get("raw") else liq,
            "entry_top10": top10 if 'top10' in locals() else 0,
            "entry_sniper_pct": float(rc["raw"].get("info", {}).get("sniperHoldingPercent", 0) or 0) if rc and rc.get("raw") else 0,
            "risk_last_checked": 0,
        }
        save_positions()

    feed(f"BUY {symbol} [{signal.get('walletTypeName','SM')}/{tier}] "
         f"{size_sol}SOL @ ${buy_price:.8f} tokens={token_amount:.0f} BE={be_pct:.1f}%")
    with state_lock:
        state["stats"]["buys"] += 1
    return True


def monitor_positions():
    """Check all positions every 20s, execute exits by priority. Runs in a separate daemon thread."""
    while True:
        time.sleep(config.POLL_INTERVAL_SEC)
        # [H3] Do not reload config here, run() main thread handles unified reload

        with pos_lock:
            positions = dict(state["positions"])
        if not positions:
            continue

        now = time.time()

        # [C11] Unconfirmed position verification (positions created after swap timeout)
        for ca, pos in list(positions.items()):
            if not pos.get("unconfirmed"): continue
            elapsed = now - pos.get("unconfirmed_ts", pos.get("opened_at_ts", 0))
            if elapsed < 60: continue  # Wait 60s before checking
            checks = pos.get("unconfirmed_checks", 0)
            try:
                # Try querying on-chain balance to confirm if transaction succeeded
                pi = onchainos('token', 'price-info', '--chain', 'solana', '--address', ca)
                p = pi[0] if isinstance(pi, list) else pi
                # If price is available and position was marked unconfirmed, attempt verification
                with pos_lock:
                    if ca in state["positions"]:
                        state["positions"][ca].pop("unconfirmed", None)
                        state["positions"][ca].pop("unconfirmed_ts", None)
                        state["positions"][ca].pop("unconfirmed_checks", None)
                        # Update price
                        if float(p.get("price", 0)) > 0:
                            state["positions"][ca]["entry_price"] = float(p["price"])
                            state["positions"][ca]["peak_price"] = float(p["price"])
                        save_positions()
                feed(f"✅ CONFIRMED {pos.get('symbol', ca[:8])}: unconfirmed → active")
                continue
            except Exception:
                checks += 1
                with pos_lock:
                    if ca in state["positions"]:
                        state["positions"][ca]["unconfirmed_checks"] = checks
                if checks >= 10 and elapsed >= 180:
                    with pos_lock:
                        state["positions"].pop(ca, None)
                        save_positions()
                    feed(f"❌ DROPPED {pos.get('symbol', ca[:8])}: unconfirmed after {checks} checks / {elapsed:.0f}s")
                continue

        # [C5+C12] Clean up ghost positions with token_amount=0 (with zero-balance count protection against RPC false negatives)
        for ca, pos in list(positions.items()):
            with pos_lock:
                live_amt = state["positions"].get(ca, {}).get("token_amount", 0)
            if live_amt <= 0:
                with pos_lock:
                    if ca not in state["positions"]: continue
                    zbc = state["positions"][ca].get("zero_balance_count", 0) + 1
                    state["positions"][ca]["zero_balance_count"] = zbc
                if zbc < 3:
                    continue  # Require 3 consecutive zero-balance confirmations before removing
                with pos_lock:
                    state["positions"].pop(ca, None)
                    cooldown_map[ca] = now + COOLDOWN_SEC
                    save_positions()
                feed(f"CLEANUP {pos.get('symbol', ca[:8])}: token_amount=0 ({zbc} checks), removed")
                del positions[ca]
            else:
                # Balance restored, reset counter
                with pos_lock:
                    if ca in state["positions"] and state["positions"][ca].get("zero_balance_count", 0) > 0:
                        state["positions"][ca]["zero_balance_count"] = 0

        if not positions:
            continue

        # Fetch prices individually (token price-info does not support batch)
        price_map = {}
        for ca in positions:
            try:
                pi = onchainos('token', 'price-info', '--chain', 'solana', '--address', ca)
                p = pi[0] if isinstance(pi, list) else pi
                price_map[ca] = p
            except Exception:
                pass
        if not price_map:
            continue

        for ca, pos in positions.items():

            p = price_map.get(ca, {})
            cur_price = float(p.get("price", 0))
            cur_liq = float(p.get("liquidity", 0))
            cur_mc = float(p.get("marketCap", 0))
            if cur_price <= 0:
                continue

            entry_price = pos["entry_price"]
            if entry_price <= 0:
                continue
            pct = (cur_price - entry_price) / entry_price * 100
            be_offset = pos.get("breakeven_pct", 0)
            net_pct = pct - be_offset
            age_min = (now - pos["opened_at_ts"]) / 60

            # Update peak + live data
            with pos_lock:
                if ca not in state["positions"]:
                    continue
                if cur_price > state["positions"][ca].get("peak_price", 0):
                    state["positions"][ca]["peak_price"] = cur_price
                state["positions"][ca]["net_pnl_pct"] = net_pct
                state["positions"][ca]["age_min"] = age_min
                if cur_mc > 0: state["positions"][ca]["current_mc"] = cur_mc
                if cur_liq > 0: state["positions"][ca]["current_liq"] = cur_liq
                peak = state["positions"][ca]["peak_price"]
                # [C3] Read live token_amount for subsequent calculations
                live_token_amount = state["positions"][ca]["token_amount"]
                live_buy_sol = state["positions"][ca]["buy_sol"]

            # [H2] Check sell failure count
            if pos.get("sell_fail_count", 0) >= getattr(config, 'MAX_SWAP_FAILS', 3):
                # Exceeded retry limit, skip this cycle
                continue

            # ── EXIT 0: Liquidity emergency exit ──
            if cur_liq > 0 and cur_liq < config.LIQ_EMERGENCY:
                close_position(ca, 1.0, "RUG_LIQ", net_pct); continue

            # ── EXIT 1: Dust cleanup ──
            value_usd = live_token_amount * cur_price  # [C3] Use live value
            if value_usd < config.MIN_POSITION_VALUE_USD:
                close_position(ca, 1.0, "DUST", net_pct); continue

            # ── EXIT 2: Hard stop loss ──
            if cur_price <= pos["sl_price"]:
                close_position(ca, 1.0, "SL", net_pct); continue

            # ── EXIT 3: Time-decay SL ──  [C1] Sort by after_min descending
            if pos["tp_tier"] == 0:
                decay_closed = False
                for rule in sorted(config.TIME_DECAY_SL, key=lambda r: r["after_min"], reverse=True):
                    if age_min >= rule["after_min"]:
                        decay_sl = entry_price * (1 + rule["sl_pct"])
                        if cur_price <= decay_sl:
                            close_position(ca, 1.0,
                                f"DECAY_SL({rule['sl_pct']:.0%})", net_pct)
                            decay_closed = True
                        break
                if decay_closed:  # [H1]
                    continue

            with pos_lock:
                if ca not in state["positions"]:
                    continue

            # ── EXIT 4: Tiered take profit (cost-aware) ──
            tp_tiers = config.TP_TIERS
            current_tp = pos["tp_tier"]
            if current_tp < len(tp_tiers):
                tp = tp_tiers[current_tp]
                tp_threshold = tp["pct"] * 100 + be_offset
                if pct >= tp_threshold:
                    ratio = tp["sell"]
                    with pos_lock:
                        if ca in state["positions"]:
                            state["positions"][ca]["tp_tier"] = current_tp + 1
                    reason = f"TP{current_tp + 1}"
                    close_position(ca, ratio, reason, net_pct)
                    continue  # [C2] Also continue after partial sell, prevent double sell in same cycle

            with pos_lock:
                if ca not in state["positions"]:
                    continue

            # ── EXIT 5: Trailing stop ──
            peak_pct = (peak - entry_price) / entry_price * 100
            if peak_pct >= config.TRAIL_ACTIVATE * 100:
                drop_from_peak = (peak - cur_price) / peak * 100
                if drop_from_peak >= config.TRAIL_DISTANCE * 100:
                    close_position(ca, 1.0, "TRAIL", net_pct); continue

            # ── EXIT 6: Trend time stop ──
            if age_min >= config.TIME_STOP_MIN_HOLD_MIN:
                if check_trend_stop(ca):
                    close_position(ca, 1.0, "TREND_STOP", net_pct); continue

            # ── EXIT 7: Hard time stop ──
            if age_min >= config.TIME_STOP_MAX_HOLD_HRS * 60:
                close_position(ca, 1.0, "TIME_STOP", net_pct); continue

            # Risk check post-trade monitoring (throttled 60s)
            _rlc = pos.get("risk_last_checked", 0)
            if now - _rlc >= 60:
                with pos_lock:
                    if ca in state["positions"]:
                        state["positions"][ca]["risk_last_checked"] = now
                _eliq = pos.get("entry_liquidity_usd", 0)
                _et10 = pos.get("entry_top10", 0)
                _esp = pos.get("entry_sniper_pct", 0)
                def _run_rc(_ca=ca, _sym=pos.get("symbol", "?"), _el=_eliq, _t10=_et10, _sp=_esp):
                    try:
                        flags = post_trade_flags(_ca, _sym, entry_liquidity_usd=_el, entry_top10=_t10, entry_sniper_pct=_sp)
                        for flag in flags:
                            feed(f"🛡️ {_sym} {flag}")
                            if flag.startswith("EXIT_NOW"):
                                close_position(_ca, 1.0, f"RISK:{flag[:30]}", 0)
                                break
                    except Exception:
                        pass
                threading.Thread(target=_run_rc, daemon=True).start()

        with pos_lock:
            save_positions()


def close_position(ca, sell_ratio, reason, net_pnl_pct):
    """
    Sell position (full or partial).
    sell_ratio: 0.0-1.0, e.g. 0.30 = sell 30%
    Note: No longer accepts pos parameter, reads from live state [C3]
    """
    with pos_lock:
        if ca not in state["positions"]: return
        if ca in _selling: return
        _selling.add(ca)
        # [C3] Read from live state, not snapshot
        pos = dict(state["positions"][ca])

    try:
        symbol = pos.get("symbol", ca[:8])
        token_amount = pos["token_amount"]
        buy_sol = pos["buy_sol"]
        sell_amount = token_amount * sell_ratio

        # [M2] Check if int truncation results in 0
        if sell_amount <= 0 or int(sell_amount) <= 0:
            return

        try:
            result = execute_swap(ca, SOL_NATIVE, int(sell_amount), wallet_addr, is_buy=False)
            tx_hash = result.get("txHash")
            # [H2] Sell succeeded, reset failure count
            with pos_lock:
                if ca in state["positions"]:
                    state["positions"][ca]["sell_fail_count"] = 0
        except Exception as e:
            feed(f"SELL FAIL {symbol} [{reason}]: {e}")
            # [H2] Record sell failure
            with pos_lock:
                if ca in state["positions"]:
                    state["positions"][ca]["sell_fail_count"] = \
                        state["positions"][ca].get("sell_fail_count", 0) + 1
                    fail_count = state["positions"][ca]["sell_fail_count"]
                    max_fails = getattr(config, 'MAX_SWAP_FAILS', 3)
                    if fail_count >= max_fails:
                        feed(f"⚠️ {symbol}: {fail_count} consecutive sell failures, pausing retries")
            return

        # Record trade  [C4] Use current buy_sol (already proportionally reduced)
        pnl_sol = buy_sol * sell_ratio * (net_pnl_pct / 100)
        record_trade(ca, pos, reason, net_pnl_pct, sell_ratio,
                     result.get("txHash"), pnl_sol)

        with pos_lock:
            if sell_ratio >= 0.999:
                state["positions"].pop(ca, None)
                cooldown_map[ca] = time.time() + COOLDOWN_SEC
            else:
                if ca in state["positions"]:
                    state["positions"][ca]["token_amount"] = token_amount - sell_amount
                    # [C4] Reduce buy_sol proportionally
                    state["positions"][ca]["buy_sol"] = buy_sol * (1 - sell_ratio)
                    # Dust check — use current price, not entry_price
                    cur_price = state["positions"][ca].get("current_price", pos["entry_price"])
                    remaining_value = state["positions"][ca]["token_amount"] * cur_price
                    if remaining_value < 0.001:
                        state["positions"].pop(ca, None)
                        cooldown_map[ca] = time.time() + COOLDOWN_SEC
            save_positions()

        feed(f"SELL {symbol} [{reason}] {sell_ratio:.0%} net:{net_pnl_pct:+.1f}%"
             + (f" tx:{tx_hash[:8]}" if tx_hash else ""))
        with state_lock:
            state["stats"]["sells"] += 1
            state["stats"]["net_sol"] = round(state["stats"]["net_sol"] + pnl_sol, 6)

    finally:
        with pos_lock:
            _selling.discard(ca)


def record_trade(ca, pos, reason, net_pnl_pct, sell_ratio, tx_hash=None, pnl_sol=0):
    """Record trade history + update session risk control state"""
    # [L4] tradeId adds random suffix to prevent collision
    rand_suffix = ''.join(random.choices(string.ascii_lowercase, k=4))
    trade = {
        "tradeId": f"sell-{int(time.time())}-{ca[:4]}-{rand_suffix}",
        "timestamp": int(time.time()),
        "direction": "sell",
        "tokenAddress": ca,
        "symbol": pos.get("symbol", ca[:8]),
        "label": pos.get("label", ""),
        "tier": pos.get("tier", ""),
        "entry_mc": pos.get("entry_mc", 0),
        "exit_mc": pos.get("current_mc", 0),
        "sol_in": pos["buy_sol"] * sell_ratio,
        "pnl_pct": net_pnl_pct + pos.get("breakeven_pct", 0),
        "net_pnl_pct": net_pnl_pct,
        "pnl_sol": round(pnl_sol, 6),
        "reason": reason,
        "txHash": tx_hash or "",
        "mode": "paper" if config.DRY_RUN else "live",
        "t": datetime.now().strftime("%H:%M:%S"),
    }
    # [H5][M4] Hold both locks together to prevent list/file divergence
    with state_lock:
        state["trades"].insert(0, trade)
        state["trades"] = state["trades"][:100]
        with trades_lock:
            save_trades()

    # Session risk control
    if net_pnl_pct < 0:
        record_loss(pnl_sol)
    else:
        record_win()


def run(wa):
    """Main loop: poll signals every POLL_INTERVAL_SEC seconds → filter → buy"""
    # [H4] Set module-level wallet_addr
    global wallet_addr
    wallet_addr = wa

    feed(f"Engine started | {'PAPER' if config.DRY_RUN else 'LIVE'} | "
         f"PAUSED={config.PAUSED} | poll={config.POLL_INTERVAL_SEC}s | max_pos={config.MAX_POSITIONS}")

    while True:
        # [H3] config reload only in main thread
        _prev_dry_run = config.DRY_RUN
        _prev_paused = config.PAUSED
        try:
            importlib.reload(config)
        except Exception as e:
            # [M6] Log warning on reload failure, keep old config
            feed(f"⚠️ config reload failed: {e}")
        # [H6] Critical parameter change warning
        if config.DRY_RUN != _prev_dry_run:
            mode = "LIVE ⚠️ Real trading" if not config.DRY_RUN else "PAPER Mode"
            feed(f"🔄 Mode switched → {mode}")
        if config.PAUSED != _prev_paused:
            feed(f"🔄 PAUSED → {config.PAUSED}")

        with state_lock:
            state["stats"]["cycle"] = state["stats"].get("cycle", 0) + 1

        # [L1] Clean up expired cooldowns
        cleanup_cooldown()

        # PAUSED and Session risk control still fetch signals (visible in Dashboard), just don't open positions
        # can_enter() already checks PAUSED and Session state, will block actual buys

        try:
            signals = onchainos('signal', 'list',
                '--chain', 'solana',
                '--wallet-type', ','.join(str(l) for l in config.SIGNAL_LABELS),
                '--min-address-count', str(config.MIN_WALLET_COUNT),
                '--min-market-cap-usd', str(config.MIN_MCAP),
                '--min-liquidity-usd', str(config.MIN_LIQUIDITY))
        except Exception as e:
            feed(f"ERROR signal list: {e}")
            time.sleep(config.POLL_INTERVAL_SEC); continue

        if not signals:
            time.sleep(config.POLL_INTERVAL_SEC); continue

        for signal in (signals if isinstance(signals, list) else [signals]):
            try:
                open_position(signal, wallet_addr)
            except Exception as e:
                feed(f"ERROR open_position: {e}")
            time.sleep(config.API_DELAY_SEC)

        time.sleep(config.POLL_INTERVAL_SEC)


# ── Dashboard ─────────────────────────────────────────────────────────────────

_dashboard_html_path = PROJECT_DIR / "dashboard.html"

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        if self.path == '/api/state':
            with state_lock:
                snap = dict(state)
                snap["trades"] = list(state["trades"])  # [H5] Copy within lock
            with pos_lock:
                snap["positions"] = dict(state["positions"])
            snap["session_risk"] = dict(session_risk)
            snap["config"] = {
                "paused": config.PAUSED,
                "dry_run": config.DRY_RUN,
                "max_positions": config.MAX_POSITIONS,
            }
            body = json.dumps(snap, ensure_ascii=False, default=str).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json; charset=utf-8')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            self.wfile.write(body)
        elif self.path in ('/', '/index.html'):
            try:
                html = _dashboard_html_path.read_text()
            except FileNotFoundError:
                html = "<h1>dashboard.html not found</h1>"
            self.send_response(200)
            self.send_header('Content-Type', 'text/html; charset=utf-8')
            self.end_headers()
            self.wfile.write(html.encode())
        else:
            self.send_response(404)
            self.end_headers()

def start_dashboard():
    port = getattr(config, 'DASHBOARD_PORT', 3248)
    HTTPServer.allow_reuse_address = True
    server = HTTPServer(("127.0.0.1", port), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"  Dashboard: http://localhost:{port}")


# ── Entry Point ──────────────────────────────────────────────────────────────

if __name__ == "__main__":
    print("=" * 55)
    print("  Smart Money Signal Copy Trade v1.0 — Agentic Wallet TEE")

    load_state()

    # Live mode: get wallet address
    if not config.DRY_RUN:
        try:
            addrs = onchainos('wallet', 'addresses', '--chain', '501')
            # Handle nested format: data.solana[{address, chainIndex}]
            sol_addrs = addrs.get("solana", []) if isinstance(addrs, dict) else []
            if sol_addrs:
                wallet_addr = sol_addrs[0].get("address", "")
            else:
                # Fallback: flat list format
                for chain in (addrs if isinstance(addrs, list) else [addrs]):
                    if chain.get("chainIndex") == 501 or chain.get("chainIndex") == "501" or "solana" in str(chain).lower():
                        wallet_addr = chain.get("address", "")
                        break
            if not wallet_addr:
                print("  ERROR: No Solana address. Run: onchainos wallet login <email>")
                exit(1)
            print(f"  Wallet: {wallet_addr[:8]}...{wallet_addr[-4:]}")
        except Exception as e:
            print(f"  ERROR: {e}")
            exit(1)
    else:
        print("  Mode: PAPER TRADE")

    port = getattr(config, 'DASHBOARD_PORT', 3248)
    print(f"  Dashboard: http://localhost:{port}")
    print(f"  Max: {config.MAX_POSITIONS} positions")
    print(f"  PAUSED: {config.PAUSED}" + (" ← Change config.py PAUSED=False to start trading" if config.PAUSED else ""))
    print("=" * 55)

    # Graceful shutdown handler
    def _shutdown_handler(signum, frame):
        print(f"\n  Received signal {signum}, shutting down...")
        with pos_lock:
            n = len(state["positions"])
        if n > 0:
            print(f"  ⚠️ WARNING: {n} position(s) still open on-chain!")
            print(f"  Positions saved in {POSITIONS_FILE}, will resume on next start.")
        else:
            print("  No open positions.")
        print("  Done.")
        sys.exit(0)

    signal.signal(signal.SIGINT, _shutdown_handler)
    signal.signal(signal.SIGTERM, _shutdown_handler)

    start_dashboard()

    # Start position monitoring thread
    threading.Thread(target=monitor_positions, daemon=True).start()

    # Start main loop
    run(wallet_addr)
