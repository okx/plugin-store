#!/usr/bin/env python3
"""
wallet_tracker.py -- 钱包跟单策略 v1.0 (Wallet Copy-Trade Bot)
监控目标钱包持仓变化，自动跟买/跟卖，onchainos CLI 驱动。

用法:
  python3 wallet_tracker.py

依赖: Python 3.8+ 标准库 + onchainos CLI >= 2.1.0
"""

import subprocess, json, os, sys, time, threading, traceback, copy
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from datetime import datetime

# ── 加载 Config ────────────────────────────────────────────────────────
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import config as C

# ── 加载 risk_check ────────────────────────────────────────────────────
try:
    from risk_check import pre_trade_checks, post_trade_flags
    HAS_RISK_CHECK = True
except ImportError:
    HAS_RISK_CHECK = False
    print("  ⚠️  risk_check.py not found -- running without risk module")

# ── Constants ──────────────────────────────────────────────────────────
_ONCHAINOS = os.path.expanduser("~/.local/bin/onchainos")
_STATE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "state")
os.makedirs(_STATE_DIR, exist_ok=True)

WALLET_ADDRESS = ""  # Set on startup

# ── State ──────────────────────────────────────────────────────────────
positions       = {}      # addr -> position dict
watch_list      = {}      # addr -> {symbol, wallet, detected_ts, mc_at_detection, ...}
wallet_snapshots = {}     # wallet_addr -> {token_addr: {amount, symbol}, ...}
tracked_tokens  = {}      # addr -> {symbol, wallet, status, ...}

pos_lock        = threading.Lock()
watch_lock      = threading.Lock()
snap_lock       = threading.Lock()
_selling        = set()
_buying         = set()
feed_lock       = threading.Lock()

# Session stats
session = {
    "start_ts":    0,
    "buys":        0,
    "sells":       0,
    "wins":        0,
    "losses":      0,
    "net_sol":     0.0,
    "consec_loss": 0,
    "paused_until": 0,
    "total_spent": 0.0,
}

trades_log   = []     # completed trade records
live_feed    = []     # dashboard feed (max 200)
_bot_running = True


# ── onchainos CLI wrapper ──────────────────────────────────────────────

def _onchainos(*args, timeout: int = 20) -> dict:
    try:
        r = subprocess.run([_ONCHAINOS, *args],
                           capture_output=True, text=True, timeout=timeout)
        result = json.loads(r.stdout) if r.stdout.strip() else {"ok": False, "data": None}
        # Log errors from CLI (previously swallowed silently)
        if not result.get("ok", True):
            err_msg = result.get("error", "")
            if err_msg:
                cmd_name = " ".join(args[:2]) if len(args) >= 2 else str(args)
                log(f"onchainos {cmd_name}: {err_msg}")
        return result
    except subprocess.TimeoutExpired:
        cmd_name = " ".join(args[:2]) if len(args) >= 2 else str(args)
        log(f"onchainos {cmd_name}: TIMEOUT ({timeout}s)")
        return {"ok": False, "data": None, "error": "timeout"}
    except json.JSONDecodeError:
        cmd_name = " ".join(args[:2]) if len(args) >= 2 else str(args)
        stderr_hint = r.stderr.strip()[:100] if r.stderr else ""
        log(f"onchainos {cmd_name}: bad JSON | stderr={stderr_hint}")
        return {"ok": False, "data": None}
    except Exception as e:
        return {"ok": False, "data": None, "error": str(e)}


def _cli_data(r: dict):
    d = r.get("data")
    if isinstance(d, list):
        return d[0] if d else {}
    return d or {}


def _cli_data_list(r: dict) -> list:
    d = r.get("data")
    return d if isinstance(d, list) else []


# ── Feed / Logging ─────────────────────────────────────────────────────

def log(msg: str):
    ts = time.strftime("%H:%M:%S")
    print(f"  [{ts}] {msg}")
    push_feed(msg)


def push_feed(msg):
    with feed_lock:
        entry = {"msg": str(msg), "t": time.strftime("%H:%M:%S")}
        live_feed.insert(0, entry)
        if len(live_feed) > 200:
            live_feed[:] = live_feed[:200]


# ── Persistence (atomic JSON writes) ──────────────────────────────────

def _atomic_write(filepath: str, data):
    os.makedirs(os.path.dirname(filepath), exist_ok=True)
    tmp = filepath + ".tmp"
    with open(tmp, "w") as f:
        json.dump(data, f, indent=2, default=str)
    os.replace(tmp, filepath)


def save_positions():
    with pos_lock:
        _atomic_write(os.path.join(_STATE_DIR, "positions.json"), positions)


def load_positions():
    global positions
    fp = os.path.join(_STATE_DIR, "positions.json")
    if os.path.exists(fp):
        try:
            with open(fp) as f:
                positions = json.load(f)
            log(f"Loaded {len(positions)} positions from disk")
        except Exception:
            positions = {}


def save_watch_list():
    with watch_lock:
        _atomic_write(os.path.join(_STATE_DIR, "tracked_tokens.json"), watch_list)


def load_watch_list():
    global watch_list
    fp = os.path.join(_STATE_DIR, "tracked_tokens.json")
    if os.path.exists(fp):
        try:
            with open(fp) as f:
                watch_list = json.load(f)
            log(f"Loaded {len(watch_list)} watched tokens from disk")
        except Exception:
            watch_list = {}


def save_snapshots():
    with snap_lock:
        _atomic_write(os.path.join(_STATE_DIR, "wallet_snapshots.json"), wallet_snapshots)


def load_snapshots():
    global wallet_snapshots
    fp = os.path.join(_STATE_DIR, "wallet_snapshots.json")
    if os.path.exists(fp):
        try:
            with open(fp) as f:
                wallet_snapshots = json.load(f)
            log(f"Loaded snapshots for {len(wallet_snapshots)} wallets")
        except Exception:
            wallet_snapshots = {}


def save_trades():
    _atomic_write(os.path.join(_STATE_DIR, "trades.json"), trades_log)


def load_trades():
    global trades_log
    fp = os.path.join(_STATE_DIR, "trades.json")
    if os.path.exists(fp):
        try:
            with open(fp) as f:
                trades_log = json.load(f)
        except Exception:
            trades_log = []


# ── onchainos Data APIs ────────────────────────────────────────────────

def get_wallet_holdings(wallet_addr: str) -> dict:
    """Returns {token_addr: {amount, symbol, name, mc, price}, ...}"""
    # onchainos >= 2.2: "all-balances --chains" for full wallet scan
    r = _onchainos("portfolio", "all-balances",
                   "--address", wallet_addr,
                   "--chains", C.CHAIN)
    # Response: data: [{"tokenAssets": [{balance, symbol, tokenContractAddress, ...}, ...]}]
    raw_data = r.get("data", [])
    items = []
    if isinstance(raw_data, list):
        for entry in raw_data:
            if isinstance(entry, dict) and "tokenAssets" in entry:
                items.extend(entry["tokenAssets"])
            elif isinstance(entry, dict) and "tokenContractAddress" in entry:
                items.append(entry)  # flat format fallback
    holdings = {}
    for item in items:
        addr = item.get("tokenContractAddress", item.get("address", ""))
        if not addr or addr in C._IGNORE_MINTS:
            continue
        try:
            amt = float(item.get("balance", item.get("holdingAmount", item.get("amount", 0))))
        except (ValueError, TypeError):
            amt = 0
        if amt <= 0:
            continue
        # Compute value from balance × tokenPrice if valueUsd missing
        value = 0.0
        try:
            value = float(item.get("valueUsd", item.get("totalValue", 0)) or 0)
            if value == 0:
                price = float(item.get("tokenPrice", 0) or 0)
                value = amt * price
        except (ValueError, TypeError):
            pass
        holdings[addr] = {
            "amount":  amt,
            "symbol":  item.get("symbol", item.get("tokenSymbol", addr[:6])),
            "name":    item.get("tokenName", item.get("name", "")),
            "value":   value,
        }
    return holdings


def price_info(token_addr: str) -> dict:
    r = _onchainos("token", "price-info",
                   "--chain", C.CHAIN, "--address", token_addr)
    items = _cli_data_list(r)
    if not items:
        items = [_cli_data(r)]
    return items[0] if items else {}


def advanced_info(token_addr: str) -> dict:
    r = _onchainos("token", "advanced-info",
                   "--chain", C.CHAIN, "--address", token_addr)
    return _cli_data(r)


def get_token_market_cap(token_addr: str) -> float:
    """
    Get the token's TOTAL market cap on the open market (price × total supply).
    NOT the wallet's holding value.

    Uses price_info first, falls back to advanced_info if needed.
    Includes sanity check: if marketCap looks like a holding value (<$50K for
    a token with liquidity), try advanced_info as fallback.
    """
    mc = 0
    source = "none"

    # Primary: price_info
    try:
        pi = price_info(token_addr)
        mc = float(pi.get("marketCap", pi.get("fdv", 0)) or 0)
        source = "price_info"
    except Exception:
        pass

    # Fallback: advanced_info (often has fdv / marketCap independently)
    if mc < 1000:
        try:
            ai = advanced_info(token_addr)
            mc2 = float(ai.get("marketCap", ai.get("fdv", ai.get("fullyDilutedValuation", 0))) or 0)
            if mc2 > mc:
                mc = mc2
                source = "advanced_info"
        except Exception:
            pass

    # Sanity: if MC is suspiciously low, try computing from price × supply
    if mc < 50_000:
        try:
            pi = price_info(token_addr) if source != "price_info" else pi
            price = float(pi.get("price", pi.get("usdPrice", 0)) or 0)
            ai = advanced_info(token_addr) if source != "advanced_info" else ai
            supply = float(ai.get("totalSupply", ai.get("circulatingSupply", 0)) or 0)
            computed_mc = price * supply
            if computed_mc > mc:
                mc = computed_mc
                source = "computed(price×supply)"
        except Exception:
            pass

    log(f"📊 MC lookup {token_addr[:8]}… = ${mc:,.0f} (via {source})")
    return mc


def get_sol_balance() -> float:
    r = _onchainos("wallet", "balance", "--chain", C.CHAIN_INDEX)
    data = _cli_data(r)
    # onchainos >= 2.2: data.details[0].tokenAssets[0].balance
    try:
        details = data.get("details", [])
        if details and isinstance(details[0], dict):
            assets = details[0].get("tokenAssets", [])
            if assets and isinstance(assets[0], dict):
                return float(assets[0].get("balance", 0) or 0)
        # Fallback: flat format
        return float(data.get("balance", data.get("totalBalance", 0)) or 0)
    except (ValueError, TypeError, IndexError):
        return 0.0


def query_single_token_balance(token_addr: str) -> float:
    """Query our own balance of a specific token."""
    if C.MODE == "paper":
        with pos_lock:
            pos = positions.get(token_addr, {})
            return pos.get("token_amount", 0)
    # Use all-balances (onchainos >= 2.2) -- response nested under tokenAssets
    r = _onchainos("portfolio", "all-balances",
                   "--address", WALLET_ADDRESS,
                   "--chains", C.CHAIN)
    raw_data = r.get("data", [])
    items = []
    if isinstance(raw_data, list):
        for entry in raw_data:
            if isinstance(entry, dict) and "tokenAssets" in entry:
                items.extend(entry["tokenAssets"])
            elif isinstance(entry, dict) and "tokenContractAddress" in entry:
                items.append(entry)
    for item in items:
        addr = item.get("tokenContractAddress", item.get("address", ""))
        if addr == token_addr:
            try:
                return float(item.get("balance", item.get("holdingAmount", item.get("amount", 0))))
            except (ValueError, TypeError):
                return 0
    return 0


# ── Safety Check ───────────────────────────────────────────────────────

def safety_check(token_addr: str, sym: str) -> tuple:
    """
    Basic safety filters before risk_check.
    Returns (passed: bool, reason: str)
    """
    try:
        pi = price_info(token_addr)
        liq = float(pi.get("liquidity", 0) or 0)
        mc  = float(pi.get("marketCap", 0) or 0)

        if liq > 0 and liq < C.MIN_LIQUIDITY:
            return False, f"LOW_LIQ ${liq:,.0f} < ${C.MIN_LIQUIDITY:,.0f}"

        if mc > C.MC_MAX_USD:
            return False, f"MC ${mc:,.0f} > cap ${C.MC_MAX_USD:,.0f}"

    except Exception as e:
        return False, f"price_info error: {e}"

    try:
        info = advanced_info(token_addr)

        # Holders: price_info has "holders", advanced_info may not
        holders = int(info.get("holderCount", info.get("holders", 0)) or 0)
        if holders == 0:
            # Fallback: get from price_info which reliably has "holders"
            holders = int(pi.get("holders", 0) or 0)
        if 0 < holders < C.MIN_HOLDERS:
            return False, f"LOW_HOLDERS {holders} < {C.MIN_HOLDERS}"

        top10 = float(info.get("top10HoldPercent", 0) or 0)
        if top10 > C.MAX_TOP10_HOLD:
            return False, f"TOP10 {top10:.0f}% > {C.MAX_TOP10_HOLD}%"

        # onchainos >= 2.2 returns "devHoldingPercent" (not "devHoldPercent")
        dev_hold = float(info.get("devHoldingPercent",
                         info.get("devHoldPercent",
                         info.get("creatorHoldPercent", 0))) or 0)
        if dev_hold > C.MAX_DEV_HOLD:
            return False, f"DEV_HOLD {dev_hold:.0f}% > {C.MAX_DEV_HOLD}%"

        bundle = float(info.get("bundleHoldingPercent", 0) or 0)
        if bundle > C.MAX_BUNDLE_HOLD:
            return False, f"BUNDLE {bundle:.0f}% > {C.MAX_BUNDLE_HOLD}%"

        rug_count = int(info.get("devRugPullTokenCount", 0) or 0)
        if rug_count > C.MAX_DEV_RUG_COUNT:
            return False, f"DEV_RUG_COUNT {rug_count} > {C.MAX_DEV_RUG_COUNT}"

    except Exception as e:
        return False, f"advanced_info error: {e}"

    return True, ""


# ── Execution: Buy ─────────────────────────────────────────────────────

def get_quote(from_addr: str, to_addr: str, amount: str, slippage: int) -> dict:
    r = _onchainos("swap", "quote", "--chain", C.CHAIN,
                   "--from", from_addr, "--to", to_addr, "--amount", amount)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})


def swap_instruction(from_addr: str, to_addr: str, amount: str,
                     slippage: int, user_wallet: str) -> dict:
    r = _onchainos("swap", "swap", "--chain", C.CHAIN,
                   "--from", from_addr, "--to", to_addr,
                   "--amount", amount,
                   "--slippage", str(slippage),
                   "--wallet", user_wallet,
                   timeout=30)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})


def sign_and_broadcast(unsigned_tx: str, to_addr: str) -> str:
    r = _onchainos("wallet", "contract-call",
                   "--chain", C.CHAIN_INDEX,
                   "--to", to_addr,
                   "--unsigned-tx", unsigned_tx,
                   "--biz-type", "dex",
                   "--strategy", "wallet-tracker-mcap",
                   timeout=60)
    data = _cli_data(r)
    if isinstance(data, list) and data:
        data = data[0]
    return data.get("txHash", "") if isinstance(data, dict) else ""


def swap_execute(from_addr: str, to_addr: str, amount: str,
                 slippage: int, user_wallet: str) -> str:
    """One-shot swap: quote → approve → swap → sign → broadcast → txHash.
    Available in onchainos >= 2.2. Returns txHash or empty string."""
    r = _onchainos("swap", "execute",
                   "--chain", C.CHAIN,
                   "--from", from_addr, "--to", to_addr,
                   "--amount", amount,
                   "--slippage", str(slippage),
                   "--wallet", user_wallet,
                   timeout=90)
    data = _cli_data(r)
    if isinstance(data, list) and data:
        data = data[0]
    if isinstance(data, dict):
        return data.get("txHash", data.get("orderId", ""))
    return ""


def tx_status(tx_hash: str) -> str:
    """Poll wallet history for tx confirmation. Returns SUCCESS/FAILED/TIMEOUT."""
    for _ in range(20):
        time.sleep(3)
        try:
            r = _onchainos("wallet", "history",
                           "--tx-hash", tx_hash,
                           "--chain-index", C.CHAIN_INDEX)
            data = _cli_data(r)
            item = data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})
            status = str(item.get("txStatus", "0"))
            if status in ("1", "2", "SUCCESS"):
                return "SUCCESS"
            if status in ("3", "FAILED"):
                return "FAILED"
        except Exception:
            pass
    return "TIMEOUT"


def can_enter(sol_amount: float) -> tuple:
    """Check if we can open a new position. Returns (ok, reason)."""
    if C.PAUSED:
        return False, "PAUSED"

    now = time.time()
    if session["paused_until"] > now:
        remaining = int(session["paused_until"] - now)
        return False, f"COOLDOWN {remaining}s"

    with pos_lock:
        if len(positions) >= C.MAX_POSITIONS:
            return False, f"MAX_POS {len(positions)}/{C.MAX_POSITIONS}"

    if session["total_spent"] + sol_amount > C.TOTAL_BUDGET:
        return False, f"BUDGET {session['total_spent']:.2f}/{C.TOTAL_BUDGET:.2f} SOL"

    if abs(session["net_sol"]) >= C.SESSION_STOP_SOL and session["net_sol"] < 0:
        return False, f"SESSION_STOP: lost {abs(session['net_sol']):.4f} SOL"

    if C.MODE != "paper":
        bal = get_sol_balance()
        if bal < C.MIN_WALLET_BAL:
            return False, f"LOW_BAL {bal:.4f} < {C.MIN_WALLET_BAL}"
        if bal - sol_amount < C.GAS_RESERVE:
            return False, f"GAS_RESERVE"

    return True, ""


def execute_buy(token_addr: str, sym: str, source_wallet: str, trigger: str):
    """Execute a buy. Called from wallet poll or MC target trigger."""
    if token_addr in _buying:
        return
    _buying.add(token_addr)

    try:
        with pos_lock:
            if token_addr in positions:
                log(f"⛔ {sym} already in positions -- skip")
                return

        ok, reason = can_enter(C.BUY_AMOUNT)
        if not ok:
            log(f"⛔ {sym} -- {reason}")
            return

        # Safety check
        safe, unsafe_reason = safety_check(token_addr, sym)
        if not safe:
            log(f"🚫 {sym} safety FAIL: {unsafe_reason}")
            return

        # risk_check pre-trade
        rc_info = {}
        if HAS_RISK_CHECK:
            try:
                rc = pre_trade_checks(token_addr, sym, quick=True)
                if rc["grade"] >= C.RISK_CHECK_GATE:
                    log(f"🛡️ {sym} RISK G{rc['grade']}: {', '.join(rc['reasons'][:2])}")
                    return
                if rc["grade"] == 2:
                    log(f"⚠️ {sym} caution: {', '.join(rc['cautions'][:2])}")
                rc_info = rc.get("raw", {}).get("info", {})
            except Exception as e:
                log(f"⚠️ {sym} risk_check error: {e}")

        # Get entry price / MC
        pi = price_info(token_addr)
        entry_price = float(pi.get("price", 0) or 0)
        entry_mc = float(pi.get("marketCap", 0) or 0)
        entry_liq = float(pi.get("liquidity", 0) or 0)

        if entry_price <= 0:
            log(f"⛔ {sym} no price data")
            return

        # Quote + Execute
        sol_lamports = str(int(C.BUY_AMOUNT * 1e9))
        if C.MODE == "paper":
            token_out = int(C.BUY_AMOUNT / entry_price) if entry_price > 0 else 0
            tx_hash = f"PAPER_{int(time.time())}"
            status = "SUCCESS"
        else:
            # Pre-check: quote for price impact
            try:
                quote = get_quote(C.SOL_ADDR, token_addr, sol_lamports, C.SLIPPAGE_BUY)
                token_out = int(quote.get("toTokenAmount", 0))
                impact = float(quote.get("priceImpactPercent",
                               quote.get("priceImpactPercentage", 100)))
                if token_out <= 0 or impact > 10:
                    log(f"⛔ {sym} bad quote (impact {impact:.1f}%)")
                    return
            except Exception as e:
                log(f"⛔ {sym} quote error: {e}")
                return

            # Primary: swap execute (one-shot, onchainos >= 2.2)
            try:
                tx_hash = swap_execute(C.SOL_ADDR, token_addr, sol_lamports,
                                       C.SLIPPAGE_BUY, WALLET_ADDRESS)
                if tx_hash:
                    log(f"📡 {sym} swap execute → {tx_hash[:16]}…")
                    status = tx_status(tx_hash)
                    if status == "FAILED":
                        log(f"❌ {sym} tx FAILED")
                        return
                else:
                    raise ValueError("swap execute returned no txHash")
            except Exception as e:
                # Fallback: 3-step flow (swap → contract-call → confirm)
                log(f"⚠️ {sym} swap execute failed ({e}), trying 3-step flow…")
                try:
                    swap = swap_instruction(C.SOL_ADDR, token_addr, sol_lamports,
                                            C.SLIPPAGE_BUY, WALLET_ADDRESS)
                    tx_obj = swap.get("tx", "")
                    unsigned_tx = tx_obj.get("data", "") if isinstance(tx_obj, dict) else tx_obj
                    if not unsigned_tx:
                        raise ValueError("Empty tx from swap")
                    tx_to = tx_obj.get("to", token_addr) if isinstance(tx_obj, dict) else token_addr
                    tx_hash = sign_and_broadcast(unsigned_tx, tx_to)
                    if not tx_hash:
                        raise ValueError("No txHash from contract-call")
                except Exception as e2:
                    log(f"❌ {sym} tx error: {e2}")
                    return

                status = tx_status(tx_hash)
                if status == "FAILED":
                    log(f"❌ {sym} tx FAILED")
                    return

        # Verify balance
        _unconfirmed = False
        if C.MODE != "paper":
            if status == "SUCCESS":
                time.sleep(2)
                actual = query_single_token_balance(token_addr)
                if actual > 0:
                    token_out = actual
            elif status == "TIMEOUT":
                time.sleep(3)
                actual = query_single_token_balance(token_addr)
                if actual > 0:
                    token_out = actual
                else:
                    _unconfirmed = True

        # Record position
        pos = {
            "symbol":       sym,
            "address":      token_addr,
            "entry":        entry_price,
            "entry_mc":     entry_mc,
            "entry_ts":     time.time(),
            "entry_human":  time.strftime("%m-%d %H:%M:%S"),
            "sol_in":       C.BUY_AMOUNT,
            "token_amount": token_out,
            "remaining":    1.0,
            "peak_price":   entry_price,
            "peak_pnl_pct": 0.0,
            "pnl_pct":      0.0,
            "current_price": entry_price,
            "sell_fails":   0,
            "stuck":        False,
            "source_wallet": source_wallet,
            "trigger":       trigger,
            "tp_tiers_hit":  [],
            "trailing_active": False,
            # risk_check snapshots
            "entry_liquidity_usd": entry_liq,
            "entry_top10":   float(rc_info.get("top10HoldPercent", 0) or 0),
            "entry_sniper_pct": float(rc_info.get("sniperHoldingPercent", 0) or 0),
            "risk_last_checked": 0,
        }
        if _unconfirmed:
            pos["unconfirmed"] = True
            pos["unconfirmed_ts"] = time.time()

        with pos_lock:
            positions[token_addr] = pos
            save_positions()

        session["buys"] += 1
        session["total_spent"] += C.BUY_AMOUNT

        # Remove from watch list if present
        with watch_lock:
            watch_list.pop(token_addr, None)
            save_watch_list()

        mode_label = "PAPER" if C.MODE == "paper" else "LIVE"
        log(f"🛒 BUY [{mode_label}] ${sym} | {C.BUY_AMOUNT} SOL @ ${entry_price:.10f} | MC ${entry_mc:,.0f} | trigger={trigger}")

    except Exception as e:
        log(f"🔴 BUY CRASH [{sym}]: {e}")
        traceback.print_exc()
    finally:
        _buying.discard(token_addr)


# ── Execution: Sell ────────────────────────────────────────────────────

def execute_sell(token_addr: str, sell_pct: float, reason: str):
    """Sell a position (full or partial)."""
    with pos_lock:
        if token_addr not in positions:
            return
        if token_addr in _selling:
            return
        _selling.add(token_addr)
        pos = copy.deepcopy(positions[token_addr])

    try:
        sym = pos.get("symbol", token_addr[:8])

        if pos.get("stuck"):
            return

        # On-chain balance
        if C.MODE == "paper":
            onchain_bal = pos.get("token_amount", 0)
        else:
            onchain_bal = query_single_token_balance(token_addr)

        if onchain_bal <= 0:
            if onchain_bal == 0:
                # Could be RPC delay -- increment zero counter
                with pos_lock:
                    if token_addr in positions:
                        zbc = positions[token_addr].get("zero_balance_count", 0) + 1
                        positions[token_addr]["zero_balance_count"] = zbc
                        if zbc >= C.ZERO_CONFIRM_COUNT:
                            # Confirmed gone
                            positions.pop(token_addr, None)
                        save_positions()
                return
            else:
                onchain_bal = pos.get("token_amount", 0)
                if onchain_bal <= 0:
                    return

        sell_amount = int(onchain_bal * min(sell_pct, 1.0))
        if sell_amount <= 0:
            return

        # Execute
        if C.MODE == "paper":
            status = "SUCCESS"
        else:
            sell_fails = pos.get("sell_fails", 0)
            dyn_slippage = C.SLIPPAGE_SELL
            if sell_fails >= 3:
                dyn_slippage = 200
            elif sell_fails >= 1:
                dyn_slippage = 100

            tx_hash = ""
            status = ""

            # Primary: swap execute (one-shot)
            try:
                tx_hash = swap_execute(token_addr, C.SOL_ADDR, str(sell_amount),
                                       dyn_slippage, WALLET_ADDRESS)
                if tx_hash:
                    log(f"📡 SELL {sym} swap execute → {tx_hash[:16]}…")
                    status = tx_status(tx_hash)
                else:
                    raise ValueError("swap execute returned no txHash (sell)")
            except Exception as e:
                # Fallback: 3-step flow
                log(f"⚠️ SELL {sym} swap execute failed ({e}), trying 3-step…")
                try:
                    swap = swap_instruction(token_addr, C.SOL_ADDR, str(sell_amount),
                                            dyn_slippage, WALLET_ADDRESS)
                    tx_obj = swap.get("tx", "")
                    unsigned_tx = tx_obj.get("data", "") if isinstance(tx_obj, dict) else tx_obj
                    if not unsigned_tx:
                        raise ValueError("Empty tx (sell)")
                    tx_to = tx_obj.get("to", C.SOL_ADDR) if isinstance(tx_obj, dict) else C.SOL_ADDR
                    tx_hash = sign_and_broadcast(unsigned_tx, tx_to)
                    if not tx_hash:
                        raise ValueError("No txHash (sell)")
                    status = tx_status(tx_hash)
                except Exception as e2:
                    log(f"❌ SELL {sym}: {e2}")
                    with pos_lock:
                        if token_addr in positions:
                            positions[token_addr]["sell_fails"] = positions[token_addr].get("sell_fails", 0) + 1
                            if positions[token_addr]["sell_fails"] >= 5:
                                positions[token_addr]["stuck"] = True
                        save_positions()
                    return

            if status == "FAILED":
                with pos_lock:
                    if token_addr in positions:
                        positions[token_addr]["sell_fails"] = positions[token_addr].get("sell_fails", 0) + 1
                    save_positions()
                return

            if status == "TIMEOUT":
                time.sleep(3)
                post_bal = query_single_token_balance(token_addr)
                if post_bal < 0 or post_bal >= onchain_bal:
                    with pos_lock:
                        if token_addr in positions:
                            positions[token_addr]["sell_fails"] = positions[token_addr].get("sell_fails", 0) + 1
                        save_positions()
                    return

        # Post-sell: check leftover
        is_partial = sell_pct < 0.99
        expected_leftover = onchain_bal - sell_amount

        if C.MODE == "paper":
            leftover = expected_leftover if is_partial else 0
        else:
            time.sleep(3)
            if is_partial and expected_leftover > 0:
                rpc = query_single_token_balance(token_addr)
                leftover = rpc if rpc > 0 else expected_leftover
            else:
                leftover = query_single_token_balance(token_addr)
                if leftover < 0:
                    leftover = max(0, expected_leftover)

        # PnL calc
        try:
            pi = price_info(token_addr)
            exit_price = float(pi.get("price", pos["entry"]))
        except Exception:
            exit_price = pos.get("current_price", pos["entry"])

        if pos["entry"] > 0:
            gross_pct = (exit_price - pos["entry"]) / pos["entry"] * 100
        else:
            gross_pct = 0.0

        sold_fraction = sell_amount / max(onchain_bal, 1)
        net_sol = pos["sol_in"] * pos["remaining"] * sold_fraction * (gross_pct / 100)

        trade_record = {
            "t":        time.strftime("%m-%d %H:%M"),
            "symbol":   sym,
            "pnl_pct":  round(gross_pct, 2),
            "sol_in":   round(pos["sol_in"] * sold_fraction, 4),
            "pnl_sol":  round(net_sol, 6),
            "reason":   reason,
            "partial":  is_partial,
            "source":   pos.get("source_wallet", "")[:8],
        }

        if leftover <= 0:
            # Full exit
            with pos_lock:
                positions.pop(token_addr, None)
                save_positions()

            trades_log.insert(0, trade_record)
            save_trades()

            session["sells"] += 1
            session["net_sol"] = round(session["net_sol"] + net_sol, 6)
            if gross_pct > 0:
                session["wins"] += 1
                session["consec_loss"] = 0
            else:
                session["losses"] += 1
                session["consec_loss"] += 1
                if session["consec_loss"] >= C.MAX_CONSEC_LOSS:
                    session["paused_until"] = time.time() + C.PAUSE_CONSEC_SEC
                    log(f"⏸️ COOLDOWN: {C.MAX_CONSEC_LOSS} consec losses → pause {C.PAUSE_CONSEC_SEC}s")

            icon = "✅" if gross_pct > 0 else "❌"
            log(f"{icon} SELL ${sym} {reason} {gross_pct:+.1f}% | {net_sol:+.6f} SOL")
        else:
            # Partial exit
            new_remaining = round(pos["remaining"] * (leftover / max(onchain_bal, 1)), 3)
            with pos_lock:
                if token_addr in positions:
                    positions[token_addr]["token_amount"] = leftover
                    positions[token_addr]["remaining"] = max(new_remaining, 0.001)
                    positions[token_addr]["sell_fails"] = 0
                save_positions()

            trades_log.insert(0, trade_record)
            save_trades()
            session["sells"] += 1
            session["net_sol"] = round(session["net_sol"] + net_sol, 6)

            log(f"✅ PARTIAL ${sym} {reason} {gross_pct:+.1f}% sold {sold_fraction:.0%}")

    except Exception as e:
        log(f"🔴 SELL CRASH [{pos.get('symbol', '?')}]: {e}")
        traceback.print_exc()
    finally:
        _selling.discard(token_addr)


# ── Wallet Poll Loop ───────────────────────────────────────────────────

def _on_wallet_buy(token_addr: str, token_info: dict, wallet_addr: str):
    """Target wallet bought a new token."""
    sym = token_info.get("symbol", token_addr[:6])
    log(f"👁️ DETECTED: {wallet_addr[:8]}… bought ${sym}")

    # Already watching or holding?
    with watch_lock:
        if token_addr in watch_list:
            return
    with pos_lock:
        if token_addr in positions:
            return

    if C.FOLLOW_MODE == "instant":
        # Buy immediately
        threading.Thread(
            target=execute_buy,
            args=(token_addr, sym, wallet_addr, "INSTANT"),
            daemon=True
        ).start()

    elif C.FOLLOW_MODE == "mc_target":
        # Add to watch list, buy when token's TOTAL market cap hits conditions.
        # Two conditions (both must be met):
        #   1. MC >= MC_TARGET_USD  (floor -- minimum total market cap)
        #   2. MC has grown >= MC_GROWTH_PCT from detection (momentum confirmation)
        # NOTE: MC = token's total market cap (price × total supply),
        #       NOT the wallet's holding value.
        try:
            current_mc = get_token_market_cap(token_addr)
        except Exception:
            current_mc = 0

        # Calculate growth target from detection MC
        growth_target_mc = current_mc * (1 + C.MC_GROWTH_PCT / 100) if C.MC_GROWTH_PCT > 0 else 0

        # Check if BOTH conditions are already met
        floor_met = current_mc >= C.MC_TARGET_USD
        growth_met = C.MC_GROWTH_PCT <= 0  # if no growth required, auto-pass

        if floor_met and growth_met:
            # Already above floor & no growth required -- buy now
            threading.Thread(
                target=execute_buy,
                args=(token_addr, sym, wallet_addr, f"MC_TARGET(already ${current_mc:,.0f})"),
                daemon=True
            ).start()
        else:
            with watch_lock:
                watch_list[token_addr] = {
                    "symbol":          sym,
                    "wallet":          wallet_addr,
                    "detected_ts":     time.time(),
                    "detected_human":  time.strftime("%m-%d %H:%M:%S"),
                    "mc_at_detection": current_mc,
                    "target_mc":       C.MC_TARGET_USD,
                    "growth_pct":      C.MC_GROWTH_PCT,
                    "growth_target":   growth_target_mc,
                }
                save_watch_list()
            parts = [f"MC now ${current_mc:,.0f} → floor ${C.MC_TARGET_USD:,.0f}"]
            if C.MC_GROWTH_PCT > 0:
                parts.append(f"need +{C.MC_GROWTH_PCT}% → ${growth_target_mc:,.0f}")
            log(f"📋 WATCH ${sym} | {' | '.join(parts)}")


def _on_wallet_sell(token_addr: str, sym: str, wallet_addr: str):
    """Target wallet sold (fully removed) a token."""
    log(f"👁️ DETECTED: {wallet_addr[:8]}… sold ${sym}")

    if not C.MIRROR_SELL:
        return

    with pos_lock:
        if token_addr not in positions:
            return

    threading.Thread(
        target=execute_sell,
        args=(token_addr, C.MIRROR_SELL_PCT, f"MIRROR_SELL({wallet_addr[:8]}…)"),
        daemon=True
    ).start()


def _on_wallet_reduce(token_addr: str, sym: str, wallet_addr: str,
                      old_amount: float, new_amount: float):
    """Target wallet partially sold a token."""
    pct_sold = (old_amount - new_amount) / old_amount
    log(f"👁️ DETECTED: {wallet_addr[:8]}… reduced ${sym} by {pct_sold:.0%}")

    if not C.MIRROR_SELL:
        return

    with pos_lock:
        if token_addr not in positions:
            return

    mirror_pct = pct_sold * C.MIRROR_SELL_PCT
    if mirror_pct < 0.05:
        return  # too small, ignore

    threading.Thread(
        target=execute_sell,
        args=(token_addr, mirror_pct, f"MIRROR_REDUCE({wallet_addr[:8]}… sold {pct_sold:.0%})"),
        daemon=True
    ).start()


def wallet_poll_loop():
    """Main loop: poll target wallets for holding changes."""
    log(f"🔄 Wallet poll started | {len(C.TARGET_WALLETS)} wallets | interval={C.POLL_INTERVAL}s")

    while _bot_running:
        try:
            for wallet_addr in C.TARGET_WALLETS:
                current = get_wallet_holdings(wallet_addr)

                with snap_lock:
                    prev = wallet_snapshots.get(wallet_addr, {})

                if not prev:
                    # First run -- just save snapshot, don't trigger buys
                    with snap_lock:
                        wallet_snapshots[wallet_addr] = current
                        save_snapshots()
                    log(f"📸 Initial snapshot for {wallet_addr[:8]}… -- {len(current)} tokens")
                    continue

                # Detect NEW buys
                for addr, info in current.items():
                    if addr not in prev:
                        _on_wallet_buy(addr, info, wallet_addr)

                # Detect SELLS (fully removed)
                for addr, info in prev.items():
                    if addr not in current:
                        _on_wallet_sell(addr, info.get("symbol", addr[:6]), wallet_addr)
                    elif current[addr]["amount"] < info["amount"] * 0.90:
                        # Partial sell (>10% reduction)
                        _on_wallet_reduce(
                            addr,
                            info.get("symbol", addr[:6]),
                            wallet_addr,
                            info["amount"],
                            current[addr]["amount"]
                        )

                # Update snapshot
                with snap_lock:
                    wallet_snapshots[wallet_addr] = current
                    save_snapshots()

        except Exception as e:
            log(f"🔴 Poll error: {e}")
            traceback.print_exc()

        time.sleep(C.POLL_INTERVAL)


# ── Monitor Loop (positions + MC targets) ──────────────────────────────

def check_mc_targets():
    """Check if any watched tokens hit their MC target (total market cap, not holding value).
    Two conditions must both be met:
      1. MC >= target_mc (floor)
      2. MC >= growth_target (mc_at_detection × (1 + growth_pct/100))
    """
    with watch_lock:
        items = list(watch_list.items())

    for addr, w in items:
        try:
            sym = w.get("symbol", addr[:6])
            mc = get_token_market_cap(addr)

            floor_mc = w.get("target_mc", C.MC_TARGET_USD)
            growth_target = w.get("growth_target", 0)
            growth_pct = w.get("growth_pct", 0)

            floor_met = mc >= floor_mc
            growth_met = growth_pct <= 0 or mc >= growth_target

            # Log progress for every watched token
            if mc > 0:
                pct_of_target = mc / max(floor_mc, 1) * 100
                log(f"👀 WATCH ${sym} MC=${mc:,.0f} / ${floor_mc:,.0f} ({pct_of_target:.0f}%) {'✅ READY' if floor_met else '⏳'}")
            else:
                log(f"⚠️ WATCH ${sym} MC=0 (API returned no data)")

            if floor_met and growth_met:
                detect_mc = w.get("mc_at_detection", 0)
                actual_growth = ((mc - detect_mc) / detect_mc * 100) if detect_mc > 0 else 0
                log(f"🎯 MC TARGET HIT: ${sym} MC=${mc:,.0f} (floor=${floor_mc:,.0f}, growth={actual_growth:+.1f}%)")
                threading.Thread(
                    target=execute_buy,
                    args=(addr, sym, w.get("wallet", ""), f"MC_TARGET(${mc:,.0f} +{actual_growth:.0f}%)"),
                    daemon=True
                ).start()

            elif mc > C.MC_MAX_USD:
                log(f"⛔ {sym} MC ${mc:,.0f} > cap -- removing from watch")
                with watch_lock:
                    watch_list.pop(addr, None)
                    save_watch_list()

        except Exception as e:
            log(f"⚠️ MC check error {addr[:8]}…: {e}")


def check_positions():
    """Check all open positions for exit conditions."""
    with pos_lock:
        addrs = list(positions.keys())

    for addr in addrs:
        with pos_lock:
            if addr not in positions:
                continue
            pos = copy.deepcopy(positions[addr])

        if pos.get("stuck"):
            continue

        sym = pos.get("symbol", addr[:8])

        # Get current price
        try:
            pi = price_info(addr)
            price = float(pi.get("price", 0) or 0)
            if price <= 0:
                continue
        except Exception:
            continue

        # Update position state
        pnl_pct = (price - pos["entry"]) / max(pos["entry"], 1e-18) * 100
        peak_price = max(pos.get("peak_price", pos["entry"]), price)
        peak_pnl = (peak_price - pos["entry"]) / max(pos["entry"], 1e-18) * 100

        with pos_lock:
            if addr in positions:
                positions[addr]["current_price"] = price
                positions[addr]["pnl_pct"] = round(pnl_pct, 2)
                positions[addr]["peak_price"] = peak_price
                positions[addr]["peak_pnl_pct"] = round(peak_pnl, 2)

        # ── STOP LOSS ──
        if pnl_pct <= C.STOP_LOSS_PCT:
            execute_sell(addr, 1.0, f"STOP_LOSS({pnl_pct:+.1f}%)")
            continue

        # ── TIME STOP ──
        age_hours = (time.time() - pos["entry_ts"]) / 3600
        if age_hours >= C.MAX_HOLD_HOURS:
            execute_sell(addr, 1.0, f"TIME_STOP({age_hours:.1f}h)")
            continue

        # ── TRAILING STOP ──
        if peak_pnl >= C.TRAILING_ACTIVATE:
            drop_from_peak = peak_pnl - pnl_pct
            with pos_lock:
                if addr in positions:
                    positions[addr]["trailing_active"] = True
            if drop_from_peak >= C.TRAILING_DROP:
                execute_sell(addr, 1.0, f"TRAILING({peak_pnl:+.1f}%→{pnl_pct:+.1f}%)")
                continue

        # ── TAKE PROFIT (tiered) ──
        for tp_pct, tp_sell in C.TP_TIERS:
            tier_key = f"tp_{tp_pct}"
            if tier_key in pos.get("tp_tiers_hit", []):
                continue
            if pnl_pct >= tp_pct:
                with pos_lock:
                    if addr in positions:
                        if "tp_tiers_hit" not in positions[addr]:
                            positions[addr]["tp_tiers_hit"] = []
                        positions[addr]["tp_tiers_hit"].append(tier_key)
                execute_sell(addr, tp_sell, f"TP+{tp_pct}%")
                break  # one TP at a time

        # ── risk_check post-trade flags ──
        if HAS_RISK_CHECK:
            now = time.time()
            if now - pos.get("risk_last_checked", 0) >= 60:
                with pos_lock:
                    if addr in positions:
                        positions[addr]["risk_last_checked"] = now

                def _check_risk_flags(a, p):
                    try:
                        flags = post_trade_flags(
                            a, p["symbol"],
                            entry_liquidity_usd=p.get("entry_liquidity_usd", 0),
                            entry_top10=p.get("entry_top10", 0),
                            entry_sniper_pct=p.get("entry_sniper_pct", 0),
                        )
                        for flag in flags:
                            log(f"🛡️ {p['symbol']} {flag}")
                            if flag.startswith("EXIT_NOW"):
                                execute_sell(a, 1.0, flag[:40])
                                break
                            elif flag.startswith("EXIT_NEXT_TP"):
                                # Tighten: sell at next opportunity
                                execute_sell(a, 1.0, flag[:40])
                                break
                    except Exception as e:
                        log(f"⚠️ risk_check post error: {e}")

                threading.Thread(target=_check_risk_flags, args=(addr, pos), daemon=True).start()

    # Save updated prices
    save_positions()


def monitor_loop():
    """Monitor positions + MC targets."""
    log(f"📊 Monitor started | interval={C.MONITOR_INTERVAL}s")

    while _bot_running:
        try:
            check_mc_targets()
            check_positions()
        except Exception as e:
            log(f"🔴 Monitor error: {e}")
            traceback.print_exc()

        time.sleep(C.MONITOR_INTERVAL)


# ── Dashboard HTTP Server ──────────────────────────────────────────────

def _dashboard_api_data() -> dict:
    """Build JSON payload for dashboard."""
    with pos_lock:
        pos_copy = copy.deepcopy(positions)
    with watch_lock:
        watch_copy = copy.deepcopy(watch_list)
    with feed_lock:
        feed_copy = list(live_feed[:100])

    return {
        "mode":          C.MODE,
        "paused":        C.PAUSED,
        "follow_mode":   C.FOLLOW_MODE,
        "mc_target_usd": C.MC_TARGET_USD,
        "mc_growth_pct": C.MC_GROWTH_PCT,
        "wallets":       [w[:8] + "…" for w in C.TARGET_WALLETS],
        "positions":     pos_copy,
        "watch_list":    watch_copy,
        "trades":        trades_log[:50],
        "feed":          feed_copy,
        "session":       session,
        "ts":            time.strftime("%H:%M:%S"),
    }


class DashboardHandler(SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/api/state":
            data = json.dumps(_dashboard_api_data(), default=str)
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(data.encode())
        elif self.path == "/" or self.path == "/index.html":
            # Serve dashboard.html
            html_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dashboard.html")
            if os.path.exists(html_path):
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                with open(html_path, "rb") as f:
                    self.wfile.write(f.read())
            else:
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.end_headers()
                self.wfile.write(b"<html><body><h1>Wallet Tracker</h1>"
                                 b"<p>dashboard.html not found</p></body></html>")
        else:
            super().do_GET()

    def do_POST(self):
        # Reject non-localhost connections for bot control endpoints
        client_ip = self.client_address[0]
        if client_ip not in ("127.0.0.1", "::1"):
            self.send_response(403)
            self.end_headers()
            self.wfile.write(b'{"error":"forbidden"}')
            return

        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length else b""
        try:
            payload = json.loads(body) if body else {}
        except Exception:
            payload = {}

        resp = {"ok": False}

        if self.path == "/api/reset-snapshot":
            # Re-baseline: snapshot current holdings → old tokens ignored from now on
            count = 0
            for wallet_addr in C.TARGET_WALLETS:
                try:
                    current = get_wallet_holdings(wallet_addr)
                    with snap_lock:
                        wallet_snapshots[wallet_addr] = current
                    count += len(current)
                    log(f"📸 RESET snapshot {wallet_addr[:8]}… → {len(current)} tokens baselined")
                except Exception as e:
                    log(f"⚠️ Reset snapshot error {wallet_addr[:8]}…: {e}")
            save_snapshots()
            resp = {"ok": True, "msg": f"Snapshots reset. {count} tokens baselined (will be ignored)."}

        elif self.path == "/api/set-mc-target":
            # Change MC_TARGET_USD at runtime
            new_mc = payload.get("mc_target", payload.get("value"))
            if new_mc is not None:
                try:
                    new_mc = int(float(new_mc))
                    if new_mc >= 100:
                        old = C.MC_TARGET_USD
                        C.MC_TARGET_USD = new_mc
                        # Update existing watch_list entries
                        with watch_lock:
                            for addr in watch_list:
                                watch_list[addr]["target_mc"] = new_mc
                            save_watch_list()
                        log(f"🎯 MC_TARGET changed: ${old:,.0f} → ${new_mc:,.0f}")
                        resp = {"ok": True, "old": old, "new": new_mc}
                    else:
                        resp = {"ok": False, "error": "mc_target must be >= 100"}
                except (ValueError, TypeError):
                    resp = {"ok": False, "error": "invalid number"}
            else:
                resp = {"ok": False, "error": "missing mc_target field"}

        elif self.path == "/api/pause":
            C.PAUSED = not C.PAUSED
            log(f"{'⏸️ PAUSED' if C.PAUSED else '▶️ RESUMED'}")
            resp = {"ok": True, "paused": C.PAUSED}

        elif self.path == "/api/clear-watch":
            # Clear watch list
            with watch_lock:
                n = len(watch_list)
                watch_list.clear()
                save_watch_list()
            log(f"🗑️ Watch list cleared ({n} tokens)")
            resp = {"ok": True, "cleared": n}

        else:
            self.send_response(404)
            self.end_headers()
            return

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(resp, default=str).encode())

    def log_message(self, format, *args):
        pass  # Suppress HTTP logs


def start_dashboard():
    try:
        server = HTTPServer(("127.0.0.1", C.DASHBOARD_PORT), DashboardHandler)
        log(f"🌐 Dashboard: http://localhost:{C.DASHBOARD_PORT}")
        server.serve_forever()
    except Exception as e:
        log(f"⚠️ Dashboard failed: {e}")


# ── Startup ────────────────────────────────────────────────────────────

def _wallet_preflight() -> str:
    """Check wallet login and return Solana address."""
    if C.MODE == "paper":
        log("📝 PAPER MODE -- no wallet needed")
        return "PAPER_MODE_NO_WALLET"

    # Check wallet status
    try:
        r = _onchainos("wallet", "status")
        data = _cli_data(r)
    except Exception as e:
        print("=" * 60)
        print("  FATAL: 无法检查 Agentic Wallet 状态")
        print(f"  错误: {e}")
        print()
        print("  请确保:")
        print("  1. onchainos CLI 已安装: onchainos --version")
        print("  2. 已登录钱包: onchainos wallet login <email>")
        print("  3. 验证状态: onchainos wallet status")
        print("=" * 60)
        sys.exit(1)

    if not data.get("loggedIn"):
        print("=" * 60)
        print("  FATAL: Agentic Wallet 未登录")
        print()
        print("  请先登录:")
        print("    onchainos wallet login <your-email>")
        print("  然后验证:")
        print("    onchainos wallet status  → loggedIn: true")
        print("=" * 60)
        sys.exit(1)

    # Get Solana address
    try:
        r2 = _onchainos("wallet", "addresses", "--chain", C.CHAIN_INDEX)
        data2 = _cli_data(r2)
    except Exception as e:
        print(f"  FATAL: 无法获取钱包地址: {e}")
        sys.exit(1)

    addr = ""
    if isinstance(data2, dict):
        sol_list = data2.get("solana", [])
        if sol_list and isinstance(sol_list[0], dict):
            addr = sol_list[0].get("address", "")
        if not addr:
            addr = data2.get("solAddress", data2.get("address", ""))
    if isinstance(data2, list) and data2:
        addr = data2[0].get("address", "") if isinstance(data2[0], dict) else str(data2[0])

    if not addr:
        print("  FATAL: 无法获取 Solana 地址")
        sys.exit(1)

    return addr


def _save_config_to_disk():
    """Write current runtime config back to config.py so it persists."""
    config_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "config.py")
    try:
        with open(config_path, "r") as f:
            lines = f.read()

        import re
        # Update TARGET_WALLETS
        wallets_str = json.dumps(C.TARGET_WALLETS)
        lines = re.sub(
            r'^TARGET_WALLETS\s*=.*$',
            f'TARGET_WALLETS    = {wallets_str}',
            lines, flags=re.MULTILINE
        )
        # Update MC_TARGET_USD
        lines = re.sub(
            r'^MC_TARGET_USD\s*=.*$',
            f'MC_TARGET_USD     = {C.MC_TARGET_USD}',
            lines, flags=re.MULTILINE
        )
        # Update MC_GROWTH_PCT
        lines = re.sub(
            r'^MC_GROWTH_PCT\s*=.*$',
            f'MC_GROWTH_PCT     = {C.MC_GROWTH_PCT}',
            lines, flags=re.MULTILINE
        )
        # Update FOLLOW_MODE
        lines = re.sub(
            r'^FOLLOW_MODE\s*=.*$',
            f'FOLLOW_MODE       = "{C.FOLLOW_MODE}"',
            lines, flags=re.MULTILINE
        )
        # Update MODE
        lines = re.sub(
            r'^MODE\s*=.*$',
            f'MODE              = "{C.MODE}"',
            lines, flags=re.MULTILINE
        )
        # Update BUY_AMOUNT
        lines = re.sub(
            r'^BUY_AMOUNT\s*=.*$',
            f'BUY_AMOUNT        = {C.BUY_AMOUNT}',
            lines, flags=re.MULTILINE
        )
        # Update PAUSED
        lines = re.sub(
            r'^PAUSED\s*=.*$',
            f'PAUSED            = {C.PAUSED}',
            lines, flags=re.MULTILINE
        )

        with open(config_path, "w") as f:
            f.write(lines)
    except Exception as e:
        print(f"  ⚠️ Could not save config: {e}")


def interactive_setup():
    """Interactive first-run setup. Asks user for wallet + params, writes to config."""
    print()
    print("=" * 60)
    print("  👁️  钱包跟单策略 v1.0 -- Wallet Copy-Trade Bot")
    print("  ── 首次启动设置 ──")
    print("=" * 60)
    print()

    # ── Q1: Target wallet(s) ──
    print("  📋 Step 1/5: 目标钱包地址")
    print("  输入你要跟踪的 Solana 钱包地址")
    print("  (多个地址用逗号分隔，直接回车跳过使用 config.py 中的值)")
    print()
    raw = input("  钱包地址 > ").strip()
    if raw:
        wallets = [w.strip() for w in raw.split(",") if len(w.strip()) > 30]
        if wallets:
            C.TARGET_WALLETS = wallets
            print(f"  ✅ 设置了 {len(wallets)} 个目标钱包")
        else:
            print("  ⚠️ 地址格式不对，请检查后重试")
            sys.exit(1)
    elif not C.TARGET_WALLETS:
        print("  ⛔ 没有设置目标钱包！请输入至少一个地址。")
        sys.exit(1)
    else:
        print(f"  ✅ 使用 config.py 中的 {len(C.TARGET_WALLETS)} 个钱包")
    print()

    # ── Q2: Follow mode ──
    print("  📋 Step 2/5: 跟单模式")
    print("  [1] ⏳ 市值触发 (MC_TARGET) -- 等代币总市值(Token MC)到目标值再买 (推荐)")
    print("  [2] ⚡ 即时跟买 (INSTANT) -- 钱包买了就跟")
    print()
    choice = input("  选择 [1/2] (默认 1) > ").strip()
    if choice == "2":
        C.FOLLOW_MODE = "instant"
        print("  ✅ 即时跟买模式")
    else:
        C.FOLLOW_MODE = "mc_target"
        print("  ✅ 市值触发模式")
    print()

    # ── Q3: MC Target (only if mc_target mode) ──
    if C.FOLLOW_MODE == "mc_target":
        # Step 3a: MC Floor
        print("  📋 Step 3a/5: 最低总市值门槛 (Token Total Market Cap)")
        print(f"  代币在市场上的总市值(价格×总供应量)低于多少不跟买？")
        print(f"  ⚠️  这是代币的总市值，不是钱包持仓市值")
        print(f"  例如: 50000, 100000, 500000 (默认 ${C.MC_TARGET_USD:,.0f})")
        print()
        raw = input(f"  最低市值 $ > ").strip().replace(",", "").replace("$", "")
        if raw:
            try:
                mc = int(float(raw))
                if mc > 0:
                    C.MC_TARGET_USD = mc
                    print(f"  ✅ 总市值 >= ${mc:,} 才跟买")
                else:
                    print(f"  ⚠️ 使用默认值 ${C.MC_TARGET_USD:,}")
            except ValueError:
                print(f"  ⚠️ 无效数字，使用默认值 ${C.MC_TARGET_USD:,}")
        else:
            print(f"  ✅ 使用默认 ${C.MC_TARGET_USD:,}")
        print()

        # Step 3b: Growth %
        print("  📋 Step 3b/5: 涨幅触发")
        print(f"  目标钱包买入后，代币总市值需要涨多少 % 才跟买？")
        print(f"  例如: 50 = 涨50%才跟, 100 = 翻倍才跟, 0 = 不等涨幅直接看市值门槛")
        print(f"  (默认 {C.MC_GROWTH_PCT}%)")
        print()
        raw = input(f"  涨幅 % > ").strip().replace("%", "")
        if raw:
            try:
                pct = float(raw)
                if 0 <= pct <= 10000:
                    C.MC_GROWTH_PCT = pct
                    if pct > 0:
                        print(f"  ✅ 需涨 {pct:.0f}% 才跟买")
                    else:
                        print(f"  ✅ 不等涨幅，只看市值门槛")
                else:
                    print(f"  ⚠️ 使用默认值 {C.MC_GROWTH_PCT}%")
            except ValueError:
                print(f"  ⚠️ 使用默认值 {C.MC_GROWTH_PCT}%")
        else:
            if C.MC_GROWTH_PCT > 0:
                print(f"  ✅ 使用默认 {C.MC_GROWTH_PCT}%")
            else:
                print(f"  ✅ 不等涨幅")
        print()
    else:
        print("  📋 Step 3/5: (即时模式跳过 MC 设置)")
        print()

    # ── Q4: Buy amount ──
    print("  📋 Step 4/5: 单笔买入金额 (SOL)")
    print(f"  每次跟买投入多少 SOL？(默认 {C.BUY_AMOUNT})")
    print()
    raw = input(f"  买入金额 (SOL) > ").strip()
    if raw:
        try:
            amt = float(raw)
            if 0.001 <= amt <= 10:
                C.BUY_AMOUNT = amt
                print(f"  ✅ 单笔 {amt} SOL")
            else:
                print(f"  ⚠️ 范围 0.001-10，使用默认 {C.BUY_AMOUNT}")
        except ValueError:
            print(f"  ⚠️ 使用默认 {C.BUY_AMOUNT} SOL")
    else:
        print(f"  ✅ 使用默认 {C.BUY_AMOUNT} SOL")
    print()

    # ── Q5: Mode ──
    print("  📋 Step 5/5: 运行模式")
    print("  [1] 🧪 模拟模式 (PAPER) -- 只看信号，不花钱 (推荐新手)")
    print("  [2] 💰 实盘模式 (LIVE) -- 真实 SOL 交易")
    print()
    choice = input("  选择 [1/2] (默认 1) > ").strip()
    if choice == "2":
        C.MODE = "live"
        print("  ✅ 实盘模式 -- 请确保钱包有足够 SOL！")
    else:
        C.MODE = "paper"
        print("  ✅ 模拟模式 -- 只观察不交易")
    print()

    # Auto-unpause since they just set everything up
    C.PAUSED = False

    # Save to config.py
    _save_config_to_disk()
    print("  💾 配置已保存到 config.py")
    print()


def main():
    global WALLET_ADDRESS, _bot_running

    print()
    print("=" * 60)
    print("  👁️  钱包跟单策略 v1.0 -- Wallet Copy-Trade Bot")
    print("=" * 60)
    print()

    # If no wallets configured, run interactive setup
    if not C.TARGET_WALLETS:
        interactive_setup()
    else:
        # Wallets already set -- ask if they want to reconfigure
        print(f"  已有配置: {len(C.TARGET_WALLETS)} 个目标钱包")
        for w in C.TARGET_WALLETS:
            print(f"    {w[:12]}…{w[-6:]}")
        print()
        choice = input("  使用现有配置启动？[Y/n] > ").strip().lower()
        if choice == "n":
            interactive_setup()
        print()

    # Final validation
    if not C.TARGET_WALLETS:
        print("  ⛔ 错误: 没有设置目标钱包！")
        sys.exit(1)

    # Wallet login
    WALLET_ADDRESS = _wallet_preflight()

    # Load persisted state
    load_positions()
    load_watch_list()
    load_snapshots()
    load_trades()

    session["start_ts"] = time.time()

    # Print config summary
    print()
    print("─" * 60)
    print("  📊 启动配置:")
    print("─" * 60)
    print(f"  模式:       {C.MODE.upper()}")
    print(f"  跟单模式:   {C.FOLLOW_MODE}")
    if C.FOLLOW_MODE == "mc_target":
        print(f"  市值门槛:   ${C.MC_TARGET_USD:,.0f} (代币总市值，非持仓市值)")
        if C.MC_GROWTH_PCT > 0:
            print(f"  涨幅触发:   +{C.MC_GROWTH_PCT:.0f}% (钱包买入后需涨此幅度)")
    print(f"  目标钱包:   {len(C.TARGET_WALLETS)} 个")
    for w in C.TARGET_WALLETS:
        print(f"              {w[:12]}…{w[-6:]}")
    print(f"  单笔买入:   {C.BUY_AMOUNT} SOL")
    print(f"  最大持仓:   {C.MAX_POSITIONS}")
    print(f"  止损:       {C.STOP_LOSS_PCT}%")
    print(f"  止盈梯度:   {C.TP_TIERS}")
    print(f"  跟卖:       {'ON' if C.MIRROR_SELL else 'OFF'} ({C.MIRROR_SELL_PCT:.0%})")
    print(f"  轮询间隔:   {C.POLL_INTERVAL}s")
    if WALLET_ADDRESS != "PAPER_MODE_NO_WALLET":
        print(f"  钱包:       {WALLET_ADDRESS[:8]}…{WALLET_ADDRESS[-6:]}")
    print(f"  Dashboard:  http://localhost:{C.DASHBOARD_PORT}")
    print()
    print("─" * 60)
    print("  🚀 启动中… Ctrl+C 停止")
    print("─" * 60)
    print()

    # Start threads
    threads = [
        threading.Thread(target=wallet_poll_loop, daemon=True, name="poll"),
        threading.Thread(target=monitor_loop, daemon=True, name="monitor"),
        threading.Thread(target=start_dashboard, daemon=True, name="dashboard"),
    ]
    for t in threads:
        t.start()

    # Main thread -- keep alive
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\n  👋 Shutting down…")
        _bot_running = False
        save_positions()
        save_watch_list()
        save_snapshots()
        save_trades()
        print("  ✅ State saved. Goodbye!")


if __name__ == "__main__":
    main()
