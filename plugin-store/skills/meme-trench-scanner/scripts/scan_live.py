"""
Meme Trench Scanner v1.0 — Agentic Wallet TEE Signing
Memepump Safety Scan + 5m/15m Precision Signals + Cost-Aware TP
Dashboard: http://localhost:3241

Run: python3 scan_live.py
Requires: onchainos CLI >= 2.1.0 (requires onchainos wallet login)
No pip install of any third-party packages needed
"""

import os, sys, time, json, subprocess, shutil, threading, random, socket, signal
from collections import defaultdict
from pathlib import Path
from http.server import HTTPServer, BaseHTTPRequestHandler

# ── Load Config ────────────────────────────────────────────────────────
PROJECT_DIR = Path(__file__).parent
sys.path.insert(0, str(PROJECT_DIR))
import config as C
from risk_check import pre_trade_checks, post_trade_flags

# ── onchainos CLI Client ───────────────────────────────────────────────

_ONCHAINOS = shutil.which("onchainos") or os.path.expanduser("~/.local/bin/onchainos")
_CHAIN = "solana"


def _check_onchainos():
    """Check if onchainos CLI is available at startup"""
    if not os.path.isfile(_ONCHAINOS):
        print("=" * 60)
        print("  FATAL: onchainos CLI not found")
        print(f"  Searched path: {_ONCHAINOS}")
        print()
        print("  Please install onchainos CLI first:")
        print("    curl -fsSL https://onchainos.com/install.sh | bash")
        print("  Or ensure onchainos is on PATH")
        print("=" * 60)
        sys.exit(1)
    try:
        r = subprocess.run([_ONCHAINOS, "--version"], capture_output=True, text=True, timeout=10)
        ver = r.stdout.strip()
        print(f"  onchainos CLI: {ver}")
    except Exception as e:
        print(f"  WARNING: onchainos --version failed: {e}")


def _onchainos(*args, timeout: int = 30) -> dict:
    """Call onchainos CLI and parse JSON output."""
    cmd = [_ONCHAINOS] + list(args)
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        raise RuntimeError(f"onchainos timeout ({timeout}s): {' '.join(args[:3])}")
    if result.returncode != 0:
        err = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"onchainos error (rc={result.returncode}): {err[:200]}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        raise RuntimeError(f"onchainos invalid JSON: {result.stdout[:200]}")


def _cli_data(resp: dict):
    """Extract .data from onchainos JSON response."""
    return resp.get("data", [])


def _safe_float(v, default=0.0):
    """Safe float conversion — handles None, empty string, non-numeric."""
    if v is None or v == "":
        return default
    try:
        return float(v)
    except (ValueError, TypeError):
        return default


def _safe_int(v, default=0):
    """Safe int conversion — handles None, empty string, non-numeric."""
    if v is None or v == "":
        return default
    try:
        return int(float(v))
    except (ValueError, TypeError):
        return default


# ── Data APIs ────────────────────────────────────────────────────────────

def token_ranking(sort_by: int) -> list:
    r = _onchainos("token", "trending", "--chain", _CHAIN,
                   "--sort-by", str(sort_by), "--time-frame", "1")
    return _cli_data(r)


def memepump_token_list(
    stage: str = "MIGRATED",
    max_mc: float = C.MC_CAP,
    min_liq: float = C.LIQ_MIN,
    min_holders: int = C.MIN_HOLDERS,
    max_bundlers_pct: float = C.TF_MAX_BUNDLERS,
    max_dev_hold_pct: float = C.DEV_HOLD_DEEP_MAX * 100,
    max_top10_pct: float = C.TOP10_HOLD_MAX,
    max_insiders_pct: float = C.INSIDERS_MAX,
    max_snipers_pct: float = C.SNIPERS_MAX,
    max_fresh_pct: float = C.FRESH_WALLET_MAX,
    limit: int = 50,
    protocol_ids: list = None,
) -> list:
    args = [
        "memepump", "tokens",
        "--chain", _CHAIN,
        "--stage", stage,
        "--max-market-cap", str(int(max_mc)),
        "--min-holders", str(min_holders),
        "--max-bundlers-percent", str(max_bundlers_pct),
        "--max-dev-holdings-percent", str(max_dev_hold_pct),
        "--max-top10-holdings-percent", str(max_top10_pct),
        "--max-insiders-percent", str(max_insiders_pct),
        "--max-snipers-percent", str(max_snipers_pct),
        "--max-fresh-wallets-percent", str(max_fresh_pct),
    ]
    if protocol_ids:
        args += ["--protocol-id-list", ",".join(protocol_ids)]
    r = _onchainos(*args)
    return _cli_data(r)


def memepump_token_details(token_address: str, wallet: str = "") -> dict:
    args = ["memepump", "token-details", "--chain", _CHAIN, "--address", token_address]
    if wallet:
        args += ["--wallet-address", wallet]
    r = _onchainos(*args)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})


_logo_cache: dict = {}

def fetch_token_logo(addr: str) -> str:
    if addr in _logo_cache:
        return _logo_cache[addr] or ""
    try:
        r = _onchainos("token", "info", "--chain", _CHAIN, "--address", addr, timeout=10)
        data = _cli_data(r)
        item = data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})
        url = item.get("logoUrl", item.get("tokenLogoUrl", ""))
        _logo_cache[addr] = url or None
        return url
    except Exception:
        _logo_cache[addr] = None
    return ""


def memepump_aped_wallet(token_address: str) -> list:
    r = _onchainos("memepump", "aped-wallet", "--chain", _CHAIN, "--address", token_address)
    return _cli_data(r)


def memepump_similar_token(token_address: str) -> list:
    r = _onchainos("memepump", "similar-tokens", "--chain", _CHAIN, "--address", token_address)
    return _cli_data(r)


def price_info(token_address: str) -> dict:
    r = _onchainos("token", "price-info", "--chain", _CHAIN, "--address", token_address)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})


def candlesticks(token_address: str, bar: str = "1m", limit: int = 20) -> list:
    r = _onchainos("market", "kline", "--chain", _CHAIN,
                   "--address", token_address, "--bar", bar, "--limit", str(limit))
    return _cli_data(r)


def trades(token_address: str, limit: int = 200) -> list:
    r = _onchainos("token", "trades", "--chain", _CHAIN,
                   "--address", token_address, "--limit", str(min(limit, 500)))
    return _cli_data(r)


# ── Safety APIs ──────────────────────────────────────────────────────────

def token_dev_info(token_address: str) -> dict:
    r = _onchainos("memepump", "token-dev-info", "--chain", _CHAIN, "--address", token_address)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})


def token_bundle_info(token_address: str) -> dict:
    r = _onchainos("memepump", "token-bundle-info", "--chain", _CHAIN, "--address", token_address)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})


def token_lp_info(token_address: str) -> dict:
    return memepump_token_details(token_address)


# ── Execution APIs ───────────────────────────────────────────────────────

def get_quote(from_addr: str, to_addr: str, amount: str, slippage: int) -> dict:
    r = _onchainos("swap", "quote", "--chain", _CHAIN,
                   "--from", from_addr, "--to", to_addr, "--amount", amount)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})


def swap_instruction(from_addr: str, to_addr: str, amount: str,
                     slippage: int, user_wallet: str) -> dict:
    # [H1] onchainos swap --slippage expects integer percent (e.g. "8" for 8%)
    r = _onchainos("swap", "swap", "--chain", _CHAIN,
                   "--from", from_addr, "--to", to_addr,
                   "--amount", amount,
                   "--slippage", str(slippage),
                   "--wallet", user_wallet,
                   timeout=30)
    data = _cli_data(r)
    return data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})


# ── Agentic Wallet (TEE Signing) ───────────────────────────────────────

def sign_and_broadcast(unsigned_tx: str, to_addr: str) -> str:
    """Sign via TEE + broadcast atomically. Returns txHash."""
    r = _onchainos("wallet", "contract-call",
                   "--chain", "501",
                   "--to", to_addr,
                   "--unsigned-tx", unsigned_tx,
                   timeout=60)
    data = _cli_data(r)
    if isinstance(data, list) and data:
        data = data[0]
    return data.get("txHash", "") if isinstance(data, dict) else ""


def tx_status(tx_hash: str) -> str:
    """Poll wallet history for tx confirmation. Returns SUCCESS/FAILED/TIMEOUT."""
    for _ in range(20):
        time.sleep(3)
        try:
            r = _onchainos("wallet", "history",
                           "--tx-hash", tx_hash,
                           "--chain-index", "501")
            data = _cli_data(r)
            item = data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})
            status = str(item.get("txStatus", "0"))
            # Compatible with two encoding schemes: gateway (2=SUCCESS,3=FAILED) and wallet (1=SUCCESS,2=FAILED)
            if status in ("1", "2", "SUCCESS"):
                return "SUCCESS"
            if status in ("3", "FAILED"):
                return "FAILED"
        except Exception:
            pass
    return "TIMEOUT"


def portfolio_token_pnl(token_address: str) -> dict:
    try:
        r = _onchainos("market", "portfolio-token-pnl",
                       "--chain", _CHAIN,
                       "--address", WALLET_ADDRESS,
                       "--token", token_address)
        data = _cli_data(r)
        return data[0] if isinstance(data, list) and data else (data if isinstance(data, dict) else {})
    except Exception:
        return {}


def _wallet_preflight() -> str:
    """Check Agentic Wallet login and return Solana address. Exits on failure."""
    if C.PAPER_TRADE:
        print("  PAPER MODE — skipping wallet login check")
        return "PAPER_MODE_NO_WALLET"

    # Check wallet status
    try:
        r = _onchainos("wallet", "status")
        data = _cli_data(r)
    except Exception as e:
        print("=" * 60)
        print("  FATAL: Unable to check Agentic Wallet status")
        print(f"  Error: {e}")
        print()
        print("  Please ensure:")
        print("  1. onchainos CLI is installed: onchainos --version")
        print("  2. Wallet is logged in: onchainos wallet login <email>")
        print("  3. Verify status: onchainos wallet status")
        print("=" * 60)
        sys.exit(1)

    if not data.get("loggedIn"):
        print("=" * 60)
        print("  FATAL: Agentic Wallet not logged in")
        print()
        print("  Please log in first:")
        print("    onchainos wallet login <your-email>")
        print("  Then verify:")
        print("    onchainos wallet status  → loggedIn: true")
        print("=" * 60)
        sys.exit(1)

    # Get Solana address
    try:
        r2 = _onchainos("wallet", "addresses", "--chain", "501")
        data2 = _cli_data(r2)
    except Exception as e:
        print(f"  FATAL: Unable to get wallet address: {e}")
        sys.exit(1)

    addr = ""
    if isinstance(data2, dict):
        sol_list = data2.get("solana", [])
        if sol_list and isinstance(sol_list[0], dict):
            addr = sol_list[0].get("address", "")
        if not addr:
            addr = data2.get("solAddress", data2.get("address", ""))
    elif isinstance(data2, list) and data2:
        item = data2[0] if isinstance(data2[0], dict) else {}
        addr = item.get("address", "")

    if not addr:
        print("  FATAL: Unable to parse Solana address")
        print("  Please check: onchainos wallet addresses --chain 501")
        sys.exit(1)

    return addr


# ── Startup Checks ─────────────────────────────────────────────────────
_check_onchainos()
WALLET_ADDRESS = _wallet_preflight()


# ── Global State ───────────────────────────────────────────────────────

state_lock = threading.Lock()
pos_lock   = threading.Lock()
_selling   = set()
_pending_buys = 0
_buy_slot_reserved = threading.local()

_last_wallet_audit = 0
_price_cache = {}  # addr → price_info dict, refreshed each monitor cycle
_WALLET_AUDIT_SEC  = 60
recently_closed    = {}
watchlist          = {}

positions = {}
state = {
    "cycle": 0, "hot": False, "status": "Starting...",
    "feed": [], "feed_seq": 0,
    "signals": [],
    "positions": {},
    "trades": [],
    "stats": {
        "cycles": 0, "buys": 0, "sells": 0, "wins": 0, "losses": 0,
        "pos_wins": 0, "pos_losses": 0,
        "net_sol": 0.0, "session_start": time.strftime("%H:%M:%S"),
    },
    "session": {
        "paused_until": None,
        "consecutive_losses": 0,
        "daily_loss_sol": 0.0,
        "stopped": False,
        "cycle_sig_a_outcomes": [],
    }
}
MAX_FEED = 600

POSITIONS_FILE       = str(PROJECT_DIR / "scan_positions.json")
TRADES_FILE          = str(PROJECT_DIR / "scan_trades.json")
RECENTLY_CLOSED_FILE = str(PROJECT_DIR / "scan_recently_closed.json")


def push_feed(row: dict):
    with state_lock:
        state["feed_seq"] += 1
        row["seq"] = state["feed_seq"]
        state["feed"].insert(0, row)
        if len(state["feed"]) > MAX_FEED:
            state["feed"] = state["feed"][:MAX_FEED]


def sync_positions():
    with pos_lock: snap = dict(positions)
    with state_lock: state["positions"] = snap


def _save_positions_unlocked():
    """Write positions to disk. Caller MUST hold pos_lock."""
    snap = dict(positions)
    try:
        with open(POSITIONS_FILE + ".tmp", "w") as f:
            json.dump(snap, f, ensure_ascii=False)
        os.replace(POSITIONS_FILE + ".tmp", POSITIONS_FILE)
    except Exception as e:
        print(f"  ⚠️ save_positions: {e}")


def save_positions():
    with pos_lock:
        _save_positions_unlocked()


def save_trades():
    with state_lock:
        snap = list(state["trades"])
    try:
        with open(TRADES_FILE + ".tmp", "w") as f:
            json.dump(snap, f, ensure_ascii=False)
        os.replace(TRADES_FILE + ".tmp", TRADES_FILE)
    except Exception as e:
        print(f"  ⚠️ save_trades: {e}")


def save_recently_closed():
    try:
        with pos_lock:
            snap = dict(recently_closed)
        with open(RECENTLY_CLOSED_FILE + ".tmp", "w") as f:
            json.dump(snap, f, ensure_ascii=False)
        os.replace(RECENTLY_CLOSED_FILE + ".tmp", RECENTLY_CLOSED_FILE)
    except Exception as e:
        print(f"  ⚠️ save_recently_closed: {e}")


# ── Balance helpers ──────────────────────────────────────────────────────

def query_all_wallet_tokens():
    """Return {mint: raw_amount} for all tokens. None on CLI error."""
    if C.PAPER_TRADE:
        return {}
    try:
        r = _onchainos("portfolio", "all-balances",
                       "--address", WALLET_ADDRESS,
                       "--chains", "solana",
                       "--filter", "1", timeout=20)
        data = _cli_data(r)
    except Exception:
        return None

    result = {}
    items = data if isinstance(data, list) else [data] if isinstance(data, dict) else []
    for item in items:
        token_assets = item.get("tokenAssets", []) if isinstance(item, dict) else []
        for t in token_assets:
            mint = t.get("tokenContractAddress", t.get("tokenAddress", ""))
            # [M2] Skip SOL (empty tokenContractAddress) and ignored mints
            if not mint or mint in C._IGNORE_MINTS:
                continue
            # [C4] Prefer rawBalance (accurate on-chain value), fallback rawAmount
            raw = t.get("rawBalance") or t.get("rawAmount") or ""
            if raw and raw not in (None, "", "0"):
                amt = int(raw)
            else:
                bal = _safe_float(t.get("balance", t.get("holdingAmount", 0)))
                decimals = _safe_int(t.get("decimals", 9), default=9)
                amt = int(bal * (10 ** decimals))
            if amt > 0:
                result[mint] = result.get(mint, 0) + amt
    return result


def query_single_token_balance(mint: str) -> int:
    """>0 = balance, 0 = confirmed empty, -1 = CLI error."""
    if C.PAPER_TRADE:
        return 0
    try:
        r = _onchainos("portfolio", "token-balances",
                       "--address", WALLET_ADDRESS,
                       "--tokens", f"501:{mint}", timeout=15)
        data = _cli_data(r)
    except Exception:
        return -1

    items = data if isinstance(data, list) else [data] if isinstance(data, dict) else []
    total = 0
    for item in items:
        token_assets = item.get("tokenAssets", []) if isinstance(item, dict) else []
        for t in token_assets:
            addr = t.get("tokenContractAddress", t.get("tokenAddress", ""))
            if addr != mint:
                continue
            # [C5] Prefer rawBalance (accurate), fallback rawAmount
            raw = t.get("rawBalance") or t.get("rawAmount") or ""
            if raw and raw not in (None, "", "0"):
                total += int(raw)
            else:
                bal = _safe_float(t.get("balance", t.get("holdingAmount", 0)))
                decimals = _safe_int(t.get("decimals", 6), default=6)
                total += int(bal * (10 ** decimals))
    return total if total > 0 else 0


def load_recently_closed():
    global recently_closed
    if os.path.exists(RECENTLY_CLOSED_FILE):
        try:
            with open(RECENTLY_CLOSED_FILE) as f:
                recently_closed = json.load(f)
            now = time.time()
            recently_closed = {a: t for a, t in recently_closed.items() if now - t <= 7200}
            print(f"  Restored {len(recently_closed)} recently_closed entries")
        except Exception as e:
            print(f"  ⚠️ load_recently_closed: {e}")


def load_on_startup():
    global positions
    if os.path.exists(POSITIONS_FILE):
        with open(POSITIONS_FILE) as f:
            positions = json.load(f)
        # Backfill origin marker for pre-fix positions
        for _addr, _pos in positions.items():
            if "origin" not in _pos:
                _pos["origin"] = "meme_trench_scanner_legacy"
        sync_positions()
        print(f"  Restored {len(positions)} positions from disk")
    if os.path.exists(TRADES_FILE):
        with open(TRADES_FILE) as f:
            with state_lock:
                state["trades"] = json.load(f)
        t_list = state["trades"]
        buys_set = set()
        sells = wins = losses = 0
        net_sol = 0.0
        pos_wins = pos_losses = 0
        daily_loss = 0.0
        for t in t_list:
            key = f"{t.get('symbol', '')}_{t.get('entry_mc', '')}"
            buys_set.add(key)
            sells += 1
            pnl_pct = t.get("pnl_pct", 0)
            sol_in = t.get("sol_in", 0)
            pnl_sol = t.get("pnl_sol", sol_in * (pnl_pct / 100))
            net_sol += pnl_sol
            if pnl_pct > 0:
                wins += 1
            else:
                losses += 1
            if not t.get("partial"):
                if pnl_pct > 0: pos_wins += 1
                else: pos_losses += 1
                if pnl_pct < 0:
                    daily_loss += abs(pnl_sol)
        with state_lock:
            state["stats"]["buys"] = len(buys_set)
            state["stats"]["sells"] = sells
            state["stats"]["wins"] = wins
            state["stats"]["losses"] = losses
            state["stats"]["net_sol"] = round(net_sol, 6)
            state["stats"]["pos_wins"] = pos_wins
            state["stats"]["pos_losses"] = pos_losses
        state["session"]["daily_loss_sol"] = round(daily_loss, 6)
        print(f"  Restored {len(t_list)} trades — {len(buys_set)} buys, net {net_sol:+.4f} SOL")
    load_recently_closed()


# ── TraderSoul ──────────────────────────────────────────────────────────
# TraderSoul system is large, loaded from a separate file
# If trader_soul_engine.py does not exist, use inline minimal implementation

SOUL_FILE = str(PROJECT_DIR / "trader_soul.json")

DEGEN_NAMES = [
    "ChadAlpha", "RugSurvivor", "DiamondPaws", "ApexApe",
    "GigaBrain", "SolSavant", "DegenLord", "MoonMathis",
    "ChaosPilot", "ZeroToHero", "BasedSatoshi", "BullishGhost",
]
STAGE_THRESHOLDS = [
    (100, 1.0,  "Legend"), (50, 0.5, "Veteran"),
    (20,  0.0,  "Seasoned"), (5, None, "Apprentice"), (0, None, "Novice"),
]

def _default_soul() -> dict:
    return {
        "name": random.choice(DEGEN_NAMES), "stage": "Novice",
        "trades_seen": 0, "wins": 0, "losses": 0, "total_pnl_sol": 0.0,
        "tier_stats": {}, "hour_stats": {},
        "personal_limits": {"bundle_ath_pct_warn": 35, "min_confidence_trust": 50},
        "win_philosophy": "I haven't found my edge yet. Every trade is a lesson.",
        "risk_philosophy": "The market owes me nothing. Protect the bag first.",
        "current_vibe": "neutral", "reflections": [], "evolution_log": [],
        "trade_outcomes": [], "periodic_reviews": [],
    }

soul = {}

def load_soul():
    global soul
    if os.path.exists(SOUL_FILE):
        try:
            with open(SOUL_FILE) as f:
                soul.update(json.load(f))
            print(f"  🧠 [{soul.get('name')}] {soul.get('stage')} — {soul.get('trades_seen',0)} trades | {soul.get('total_pnl_sol',0):+.4f} SOL")
        except Exception as e:
            print(f"  ⚠️ Soul load error: {e} — starting fresh")
            soul.update(_default_soul())
    else:
        soul.update(_default_soul())
        _save_soul()
        print(f"  🧠 TraderSoul born: [{soul['name']}]")

def _save_soul():
    try:
        tmp = SOUL_FILE + ".tmp"
        with open(tmp, "w") as f:
            json.dump(soul, f, ensure_ascii=False, indent=2)
        os.replace(tmp, SOUL_FILE)
    except Exception:
        pass

def _add_reflection(text: str):
    entry = {"t": time.strftime("%H:%M:%S"), "msg": text}
    soul.setdefault("reflections", []).insert(0, entry)
    soul["reflections"] = soul["reflections"][:10]
    push_feed({"sym_note": True, "msg": f"🧠 {soul.get('name','?')}: {text}", "t": time.strftime("%H:%M:%S")})

def reflect_on_signal(sym, tier, confidence):
    soul["signals_seen"] = soul.get("signals_seen", 0) + 1
    _add_reflection(f"{sym} — {tier} signal. Confidence {confidence:.0f}.")
    if soul["signals_seen"] % 5 == 0:
        _evolve_philosophy()
    _save_soul()

def reflect_on_entry(sym, tier, sol_in, confidence):
    _add_reflection(f"Entered {sym} at {sol_in:.3f} SOL. Confidence {confidence:.0f}.")
    _save_soul()

def reflect_on_exit(sym, tier, pnl_sol, reason, hold_min):
    is_win = pnl_sol > 0
    soul["trades_seen"] = soul.get("trades_seen", 0) + 1
    soul["total_pnl_sol"] = round(soul.get("total_pnl_sol", 0) + pnl_sol, 6)
    if is_win: soul["wins"] = soul.get("wins", 0) + 1
    else:      soul["losses"] = soul.get("losses", 0) + 1

    ts = soul.setdefault("tier_stats", {})
    t = ts.setdefault(tier, {"wins": 0, "losses": 0, "n": 0, "rate": 0.5})
    if is_win: t["wins"] += 1
    else:      t["losses"] += 1
    t["n"] = t["wins"] + t["losses"]
    t["rate"] = round(t["wins"] / t["n"], 3) if t["n"] > 0 else 0.5

    hs = soul.setdefault("hour_stats", {})
    h = hs.setdefault(str(int(time.strftime("%H"))), {"wins": 0, "losses": 0, "n": 0, "rate": 0.5})
    if is_win: h["wins"] += 1
    else:      h["losses"] += 1
    h["n"] = h["wins"] + h["losses"]
    h["rate"] = round(h["wins"] / h["n"], 3) if h["n"] > 0 else 0.5

    soul.setdefault("trade_outcomes", []).insert(0, {
        "sym": sym, "tier": tier, "pnl": round(pnl_sol, 6),
        "reason": reason, "hold_min": round(hold_min, 1),
        "t": time.strftime("%H:%M:%S"), "win": is_win,
    })
    soul["trade_outcomes"] = soul["trade_outcomes"][:20]

    if is_win:
        _add_reflection(f"{sym} +{pnl_sol:.4f} SOL via {reason}.")
    else:
        _add_reflection(f"{sym} {pnl_sol:.4f} SOL via {reason}.")

    _update_stage()
    _save_soul()

def _evolve_philosophy():
    wins = soul.get("wins", 0)
    losses = soul.get("losses", 0)
    total = wins + losses
    if total < 10: return
    win_rate = wins / total
    pnl = soul.get("total_pnl_sol", 0)
    if win_rate >= 0.65: soul["current_vibe"] = "euphoric"
    elif win_rate >= 0.50: soul["current_vibe"] = "bullish"
    elif win_rate >= 0.40: soul["current_vibe"] = "neutral"
    else: soul["current_vibe"] = "paranoid"
    soul["win_philosophy"] = f"{win_rate*100:.0f}% WR over {total} trades. PnL {pnl:+.4f} SOL."

def _update_stage():
    t_count = soul.get("trades_seen", 0)
    pnl = soul.get("total_pnl_sol", 0)
    for min_trades, min_pnl, stage in STAGE_THRESHOLDS:
        if t_count >= min_trades and (min_pnl is None or pnl >= min_pnl):
            if soul.get("stage") != stage:
                push_feed({"sym_note": True, "msg": f"🌟 [{soul.get('name')}] → {stage}!", "t": time.strftime("%H:%M:%S")})
            soul["stage"] = stage
            return

def soul_summary() -> dict:
    return {
        "name": soul.get("name", "?"), "stage": soul.get("stage", "Novice"),
        "trades": soul.get("trades_seen", 0),
        "win_rate": round(soul.get("wins", 0) / max(soul.get("trades_seen", 1), 1), 3),
        "pnl_sol": soul.get("total_pnl_sol", 0),
        "vibe": soul.get("current_vibe", "neutral"),
        "win_philosophy": soul.get("win_philosophy", ""),
        "risk_philosophy": soul.get("risk_philosophy", ""),
        "reflections": soul.get("reflections", [])[:8],
        "tier_stats": soul.get("tier_stats", {}),
        "wins": soul.get("wins", 0), "losses": soul.get("losses", 0),
    }


# ── Session Risk Control ───────────────────────────────────────────────

def can_enter(sol_amount: float, reserve: bool = False):
    global _pending_buys
    if C.PAUSED:
        return False, "PAUSED (manual)"
    with state_lock:
        s = state["session"]
        if s["stopped"]:
            return False, "Session stopped"
        if C.MAX_TRADES and state["stats"]["buys"] >= C.MAX_TRADES:
            s["stopped"] = True
            push_feed({"sym_note": True, "msg": f"🏁 MAX_TRADES ({C.MAX_TRADES}) reached", "t": time.strftime("%H:%M:%S")})
            return False, f"MAX_TRADES ({C.MAX_TRADES})"
        if s["paused_until"] and time.time() < s["paused_until"]:
            return False, f"Paused — {int((s['paused_until']-time.time())/60)}min left"
    # HKT sleep 04:00-08:00
    import datetime as _dt
    _hkt_hour = _dt.datetime.now(_dt.timezone(_dt.timedelta(hours=8))).hour
    if 4 <= _hkt_hour < 8:
        return False, f"Sleep (04-08 HKT), now {_hkt_hour:02d}:xx"
    with pos_lock:
        effective = len(positions) + _pending_buys
        if effective >= C.MAX_POSITIONS:
            return False, "Max positions"
        total_exp = sum(p.get("sol_in", 0) for p in positions.values())
        if total_exp + sol_amount > C.MAX_SOL:
            return False, f"Exposure cap"
        if reserve:
            _pending_buys += 1

    # Wallet SOL balance check (live mode only, fail-open on query error)
    if not C.PAPER_TRADE:
        try:
            bal_data = _onchainos("wallet", "balance", "--chain", "501", timeout=10)
            data = _cli_data(bal_data)
            sol_bal = 0.0
            # Parse nested structure: data.details[0].tokenAssets[] to find native SOL (tokenAddress is empty)
            if isinstance(data, dict):
                details = data.get("details", [])
                if isinstance(details, list) and details:
                    for asset in details[0].get("tokenAssets", []):
                        ta = asset.get("tokenAddress", asset.get("tokenContractAddress", ""))
                        if ta in ("", None):
                            sol_bal = _safe_float(asset.get("balance", 0))
                            break
            if sol_bal < sol_amount + C.SOL_GAS:
                if reserve:
                    with pos_lock:
                        _pending_buys = max(0, _pending_buys - 1)
                return False, f"SOL balance {sol_bal:.4f} < {sol_amount + C.SOL_GAS:.4f}"
        except Exception:
            pass  # Fail-open: exposure cap already provides base protection

    return True, "OK"


def record_loss(net_sol: float):
    with state_lock:
        s = state["session"]
        s["consecutive_losses"] += 1
        s["daily_loss_sol"] = round(s["daily_loss_sol"] + abs(net_sol), 6)
        if s["daily_loss_sol"] >= C.STOP_LOSS_SOL:
            s["stopped"] = True
            push_feed({"sym_note": True, "msg": f"🛑 STOPPED — loss {s['daily_loss_sol']:.3f} SOL", "t": time.strftime("%H:%M:%S")})
            return
        if s["consecutive_losses"] >= C.MAX_CONSEC_LOSS:
            s["paused_until"] = time.time() + C.PAUSE_CONSEC_SEC
            push_feed({"sym_note": True, "msg": f"⏸ Paused {C.PAUSE_CONSEC_SEC//60}min", "t": time.strftime("%H:%M:%S")})
        elif s["daily_loss_sol"] >= C.PAUSE_LOSS_SOL:
            s["paused_until"] = time.time() + 1800
            push_feed({"sym_note": True, "msg": f"⏸ Paused 30min — loss {s['daily_loss_sol']:.3f} SOL", "t": time.strftime("%H:%M:%S")})


def record_win():
    with state_lock:
        state["session"]["consecutive_losses"] = 0


# ── Pre-Filter ──────────────────────────────────────────────────────────

def pre_filter(candidates: list, now_sec: float) -> list:
    survivors = []
    for token in candidates:
        mkt = token.get("market", {})
        tags = token.get("tags", {})
        sym = token.get("symbol", token.get("tokenContractAddress", "?")[:8])
        mc = float(mkt.get("marketCapUsd", 0) or 0)
        buys = int(float(mkt.get("buyTxCount1h", 0) or 0))
        sells = max(int(float(mkt.get("sellTxCount1h", 1) or 1)), 1)
        bs = buys / sells
        vol1h = float(mkt.get("volumeUsd1h", 0) or 0)
        created_ms = float(token.get("createdTimestamp", str(int(now_sec * 1000))) or str(int(now_sec * 1000)))
        age = now_sec - created_ms / 1000
        dev_pct = float(tags.get("devHoldingsPercent", -1) or -1)
        dev = dev_pct / 100 if dev_pct >= 0 else -1
        holders = int(float(tags.get("totalHolders", -1) or -1))

        if mc > C.MC_CAP or mc < C.MC_MIN: continue
        if bs < C.BS_MIN: continue
        if age < C.AGE_HARD_MIN or age > C.AGE_MAX: continue
        if dev > 0.05: continue
        if vol1h < C.TF_MIN_VOLUME: continue
        if mc > 0 and vol1h / mc < C.VOLMC_MIN_RATIO: continue
        if holders >= 0 and holders < C.MIN_HOLDERS: continue

        token["_sym"] = sym
        token["_age"] = age
        token["_bs"] = bs
        token["_vol1h"] = vol1h
        token["_mc"] = mc
        token["_early_window"] = age < C.AGE_SOFT_MIN
        token["_dev_flag"] = f"DEV {dev*100:.0f}%" if dev >= 0 else "DEV N/A"
        survivors.append(token)
    return survivors


# ── Safety Check ────────────────────────────────────────────────────────

def check_dev_sell(candles: list):
    if not candles or len(candles) < 4:
        return False, ""
    highs = [float(c["h"]) for c in candles]
    ath = max(highs)
    live_close = float(candles[0]["c"])
    if ath > 0:
        drawdown_pct = (ath - live_close) / ath * 100
        if drawdown_pct >= C.DEV_SELL_DROP_PCT:
            return True, f"ATH_DROP {drawdown_pct:.0f}%"
    return False, ""


def _fetch_safety_data(addr: str, sym: str) -> dict:
    result = {
        "audit_score": -1, "lp_pct": -1.0, "lp_burned": False,
        "rug_count": 0, "rug_rate": 0.0, "dev_hold": 0.0,
        "dev_launched": 0, "bundle_ath": 0.0, "bundle_count": 0,
        "aped_count": 0, "dev_serial_rug": False, "dev_death_rate": 0.0,
        "warnings": []
    }
    try:
        details = memepump_token_details(addr)
        result["audit_score"] = float(details.get("auditScore", details.get("score", -1)))
        raw_lp = float(details.get("lpLockedPercent", details.get("lpLockPercent", -1)))
        if raw_lp >= 0:
            result["lp_pct"] = raw_lp if raw_lp <= 1 else raw_lp / 100
        result["lp_burned"] = bool(details.get("lpBurned", details.get("isLpBurned", False)))
    except Exception as e:
        result["warnings"].append(f"tokenDetails: {e}")
    try:
        dev_info = token_dev_info(addr)
        # [C1] API returns nested: {devHoldingInfo: {...}, devLaunchedInfo: {...}}
        holding = dev_info.get("devHoldingInfo", {}) if isinstance(dev_info, dict) else {}
        launched = dev_info.get("devLaunchedInfo", {}) if isinstance(dev_info, dict) else {}
        result["rug_count"] = _safe_int(launched.get("rugPullCount", 0))
        total_tokens = _safe_int(launched.get("totalTokens", 0))
        # [H2] rug_rate not returned by API — compute from rugPullCount/totalTokens
        result["rug_rate"] = result["rug_count"] / max(total_tokens, 1)
        # [H3] API returns percent number (e.g. 98.705), config expects decimal (0.10)
        result["dev_hold"] = _safe_float(holding.get("devHoldingPercent", 0)) / 100
        result["dev_launched"] = total_tokens
    except Exception as e:
        result["warnings"].append(f"devInfo: {e}")
    try:
        bundle = token_bundle_info(addr)
        # [C2] API returns empty strings — use safe conversion; field is totalBundlers not bundlerCount
        result["bundle_ath"] = _safe_float(bundle.get("bundlerAthPercent", 0))
        result["bundle_count"] = _safe_int(bundle.get("totalBundlers", bundle.get("bundlerCount", 0)))
    except Exception as e:
        result["warnings"].append(f"bundleInfo: {e}")
    try:
        aped = memepump_aped_wallet(addr)
        result["aped_count"] = len(aped)
    except Exception as e:
        result["warnings"].append(f"apedWallet: {e}")
    try:
        similar = memepump_similar_token(addr)
        if similar and len(similar) >= 3:
            dead = sum(1 for s in similar
                       if float(s.get("marketCap", s.get("marketCapUsd", 0)) or 0) < 1000
                       or s.get("isRugPull", s.get("rugPull", False)))
            result["dev_death_rate"] = dead / len(similar)
            result["dev_serial_rug"] = result["dev_death_rate"] > 0.60
    except Exception as e:
        result["warnings"].append(f"similarToken: {e}")
    return result


def deep_safety_check(addr: str, sym: str):
    d = _fetch_safety_data(addr, sym)
    if d["audit_score"] >= 0 and d["audit_score"] < 30:
        return False, f"AUDIT {d['audit_score']:.0f}"
    # Rate-based rug check (aligned with risk_check.py)
    if d["rug_rate"] >= 0.20 and d["rug_count"] >= 3:
        return False, f"SERIAL_RUGGER rate={d['rug_rate']*100:.0f}% ×{d['rug_count']}"
    # Absolute count fallback (configurable)
    max_rug = getattr(C, 'MAX_DEV_RUG_COUNT', 5)
    if max_rug and d["rug_count"] > max_rug:
        return False, f"DEV_RUG ×{d['rug_count']}"
    if d["dev_hold"] > C.DEV_HOLD_DEEP_MAX:
        return False, f"DEV_HOLD {d['dev_hold']*100:.0f}%"
    if C.DEV_MAX_LAUNCHED and d["dev_launched"] > C.DEV_MAX_LAUNCHED:
        return False, f"SERIAL_DEV {d['dev_launched']}"
    if d["dev_serial_rug"]:
        return False, f"SERIAL_RUG {d['dev_death_rate']*100:.0f}%"
    if d["bundle_ath"] > C.BUNDLE_ATH_PCT_MAX:
        return False, f"BUNDLE_ATH {d['bundle_ath']:.0f}%"
    if C.BUNDLE_MAX_COUNT and d["bundle_count"] > C.BUNDLE_MAX_COUNT:
        return False, f"BUNDLE_CNT {d['bundle_count']}"
    if d["aped_count"] > C.APED_WALLET_MAX:
        return False, f"APED {d['aped_count']}"
    if C.LP_LOCK_MIN_PCT > 0:
        if d["lp_burned"]:
            pass
        elif d["lp_pct"] >= 0 and d["lp_pct"] < C.LP_LOCK_MIN_PCT:
            return False, f"LP_UNLOCK {d['lp_pct']*100:.0f}%"
        elif C.LP_LOCK_STRICT and d["lp_pct"] < 0:
            return False, "LP_FAIL"
    return True, "OK"


# ── Signal Detection ────────────────────────────────────────────────────

def detect_signal(token: dict) -> dict:
    sym = token["_sym"]
    addr = token.get("tokenContractAddress", token.get("tokenAddress", ""))
    now = time.strftime("%H:%M:%S")

    ratio_c_1h = token["_bs"]
    if ratio_c_1h < 1.0:
        return {"symbol": sym, "addr": addr, "tier": "NO_SIGNAL", "sig_a": False, "sig_b": False, "sig_c": False, "t": now}

    hot = state["session"].get("hot_mode", False)
    SIG_A = 1.2 if hot else C.SIG_A_THRESHOLD

    try:
        raw_trades = trades(addr, limit=200)
    except Exception as e:
        return {"symbol": sym, "addr": addr, "tier": "ERROR", "err": str(e), "t": now}

    # 5m/15m B/S
    now_ms = int(time.time() * 1000)
    buys_5m = sells_5m = buys_15m = sells_15m = 0
    for t in raw_trades:
        t_ms = int(t.get("time", 0))
        age_ms = now_ms - t_ms
        side = t.get("type", "")
        if age_ms <= 15 * 60 * 1000:
            if side == "buy": buys_15m += 1
            elif side == "sell": sells_15m += 1
            if age_ms <= 5 * 60 * 1000:
                if side == "buy": buys_5m += 1
                elif side == "sell": sells_5m += 1
    ratio_c_5m = buys_5m / max(sells_5m, 1)
    ratio_c_15m = buys_15m / max(sells_15m, 1)
    ratio_c = max(ratio_c_5m, ratio_c_15m)
    sig_c = ratio_c >= 1.5
    if not sig_c:
        return {"symbol": sym, "addr": addr, "tier": "NO_SIGNAL", "sig_a": False, "sig_b": False, "sig_c": False,
                "ratio_c": round(ratio_c, 2), "t": now}

    # Anti-chase
    if len(raw_trades) >= 5:
        try:
            p_new = float(raw_trades[0].get("price", 0))
            p_old = float(raw_trades[-1].get("price", p_new))
            if p_old > 0 and p_new / p_old > 2.0:
                return {"symbol": sym, "addr": addr, "tier": "NO_SIGNAL", "sig_a": False, "sig_b": False, "sig_c": True,
                        "ratio_c": round(ratio_c, 2), "t": now}
        except (TypeError, ValueError, ZeroDivisionError):
            pass

    # Signal A — TX Acceleration
    minute_counts = defaultdict(int)
    for t in raw_trades:
        minute_counts[(int(t["time"]) // 1000 // 60) * 60] += 1
    sorted_mins = sorted(minute_counts.keys())

    sig_a = False
    signal_a_ratio = 0
    if len(sorted_mins) >= 2:
        curr_min = sorted_mins[-1]
        prev_min = sorted_mins[-2]
        curr_time = max(int(t["time"]) for t in raw_trades) // 1000
        elapsed = max(curr_time - curr_min, 1)
        curr_count = minute_counts[curr_min]
        prev_count = minute_counts[prev_min]
        projected = (curr_count / elapsed) * 60
        if prev_count > 0:
            signal_a_ratio = projected / prev_count
        sig_a = (curr_count >= 10 and signal_a_ratio >= SIG_A) or (curr_count >= 10 and projected >= C.SIG_A_FLOOR_TXS_MIN)

    state["session"].setdefault("cycle_sig_a_outcomes", []).append(
        (minute_counts.get(sorted_mins[-1] if sorted_mins else 0, 0), signal_a_ratio)
    )

    if not sig_a:
        return {"symbol": sym, "addr": addr, "tier": "NO_SIGNAL",
                "sig_a": False, "sig_a_ratio": round(signal_a_ratio, 2),
                "sig_b": False, "sig_c": True, "ratio_c": round(ratio_c, 2), "t": now}

    # Signal B — Candles
    try:
        candles_data = candlesticks(addr, bar="1m", limit=20)
    except Exception as e:
        return {"symbol": sym, "addr": addr, "tier": "ERROR", "err": str(e), "t": now}
    if not candles_data:
        return {"symbol": sym, "addr": addr, "tier": "NO_SIGNAL", "sig_a": True, "sig_b": False, "sig_c": True, "t": now}

    live = candles_data[0]
    live_drop = (float(live["c"]) - float(live["o"])) / max(float(live["o"]), 1e-12) * 100
    if live_drop <= -30:
        return {"symbol": sym, "addr": addr, "tier": "DEV_SELL", "t": now}

    dev_sold, _ = check_dev_sell(candles_data)
    if dev_sold:
        return {"symbol": sym, "addr": addr, "tier": "DEV_SELL", "t": now}

    # Price position filter
    highs = [float(c["h"]) for c in candles_data[:20]]
    lows = [float(c["l"]) for c in candles_data[:20]]
    range_high, range_low = max(highs), min(lows)
    price_position = (float(candles_data[0]["c"]) - range_low) / max(range_high - range_low, 1e-12)
    if price_position >= 0.85:
        return {"symbol": sym, "addr": addr, "tier": "TOP_ZONE", "price_position": round(price_position, 3), "t": now}

    launch_vol = float(candles_data[-1].get("vol", 0))
    launch_type = "hot" if launch_vol > 150_000_000 else "quiet"
    curr_5m_vol = sum(float(c["vol"]) for c in candles_data[:5])

    if launch_type == "quiet":
        baseline = sum(float(c["vol"]) for c in candles_data[:20]) / max(len(candles_data[:20]) / 5, 1)
        sig_b = curr_5m_vol > 1.5 * baseline if baseline > 0 else False
        sig_b_ratio = curr_5m_vol / baseline if baseline > 0 else 0
    else:
        baseline = sum(float(c["vol"]) for c in candles_data[:10]) / max(len(candles_data[:10]) / 5, 1)
        consec_up = len(candles_data) >= 3 and float(candles_data[0]["c"]) > float(candles_data[1]["c"]) > float(candles_data[2]["c"])
        sig_b = curr_5m_vol > 1.2 * baseline and consec_up if baseline > 0 else False
        sig_b_ratio = curr_5m_vol / baseline if baseline > 0 else 0

    if sig_a and sig_b and sig_c: tier = "STRONG"
    elif sig_a and sig_c: tier = "MINIMUM"
    else: tier = "NO_SIGNAL"

    # Stairstep upgrade
    stairstep = False
    if len(candles_data) >= 4:
        stairstep = all(float(candles_data[i]["c"]) > float(candles_data[i+1]["c"]) for i in range(3))
    # Stairstep upgrade: NO_SIGNAL → MINIMUM when sig_a+sig_c present and price stairstepping
    if stairstep and tier == "NO_SIGNAL" and sig_a and sig_c:
        tier = "MINIMUM"

    # Confidence
    conf = 0
    if sig_a:
        if signal_a_ratio >= 3.0: conf += 35
        elif signal_a_ratio >= 2.0: conf += 25
        else: conf += 15
    if sig_c:
        if ratio_c >= 2.0: conf += 15
        else: conf += 10
    if sig_b: conf += 15
    if stairstep: conf += 15
    if token.get("_early_window"): conf += 10
    mc_est = token.get("_mc", 0)
    vol1h_est = token.get("_vol1h", 0)
    if mc_est > 0 and vol1h_est / mc_est >= 0.20: conf += 5
    conf = min(conf, 100)

    entry_price = float(candles_data[0]["c"])

    return {
        "symbol": sym, "addr": addr, "tier": tier, "launch": launch_type,
        "sig_a": sig_a, "sig_a_ratio": round(signal_a_ratio, 2),
        "sig_b": sig_b, "sig_b_ratio": round(sig_b_ratio, 2),
        "sig_c": sig_c, "ratio_c": round(ratio_c, 2),
        "entry": entry_price, "mc": mc_est,
        "age_m": round(token["_age"] / 60, 1),
        "confidence": conf, "stairstep": stairstep,
        "price_position": round(price_position, 3),
        "near_migration": float(token.get("bondingPercent", 0)) >= C.BOND_NEAR_PCT,
        "needs_pullback": False,
        "t": now,
    }


# ── Hot Mode ────────────────────────────────────────────────────────────

def hot_mode_check():
    outcomes = state["session"].get("cycle_sig_a_outcomes", [])
    if outcomes:
        born_running = sum(1 for (cc, r) in outcomes if cc > 30 and r < 1.5)
        ratio = born_running / len(outcomes)
        prev = state["session"].get("hot_mode", False)
        state["session"]["hot_mode"] = ratio > C.HOT_MODE_RATIO
        if state["session"]["hot_mode"] and not prev:
            push_feed({"sym_note": True, "msg": "🌶️ HOT MODE ON", "t": time.strftime("%H:%M:%S")})
        elif not state["session"]["hot_mode"] and prev:
            push_feed({"sym_note": True, "msg": "❄️ Hot Mode OFF", "t": time.strftime("%H:%M:%S")})
    state["session"]["cycle_sig_a_outcomes"] = []


# ── Buy Execution ───────────────────────────────────────────────────────

def _try_open_position_inner(result: dict):
    global _pending_buys
    sym = result["symbol"]
    addr = result["addr"]
    tier = result["tier"]
    launch = result.get("launch", "quiet")
    conf = result.get("confidence", 0)
    sol_amount = C.SOL_PER_TRADE.get(tier, 0.01)
    slippage = C.SLIPPAGE_BUY.get(tier, 10)

    if addr in C._NEVER_TRADE_MINTS: return
    with pos_lock:
        if addr in positions: return
        if addr in recently_closed: return

    existing_bal = query_single_token_balance(addr)
    if existing_bal > 0:
        push_feed({"sym_note": True, "msg": f"⛔ {sym} already in wallet — skip", "t": time.strftime("%H:%M:%S")})
        return

    ok, reason = can_enter(sol_amount, reserve=True)
    if not ok:
        push_feed({"sym_note": True, "msg": f"⛔ {sym} — {reason}", "t": time.strftime("%H:%M:%S")})
        return
    _buy_slot_reserved.flag = True

    # Liquidity check
    try:
        pi = price_info(addr)
        liq = float(pi.get("liquidity", 0))
        if liq > 0 and liq < C.LIQ_MIN:
            push_feed({"sym_note": True, "msg": f"⛔ {sym} liq ${liq/1000:.1f}K", "t": time.strftime("%H:%M:%S")})
            return
        entry_price = float(pi.get("price", result.get("entry", 0)))
    except Exception as e:
        push_feed({"sym_note": True, "msg": f"⛔ {sym} price-info: {e}", "t": time.strftime("%H:%M:%S")})
        return

    # Deep safety
    safe, unsafe_reason = deep_safety_check(addr, sym)
    if not safe:
        push_feed({"sym_note": True, "msg": f"🚫 {sym} — {unsafe_reason}", "t": time.strftime("%H:%M:%S")})
        return

    # Risk check — honeypot, wash trading, rug rate (v1.1)
    _rc_info = {}
    try:
        rc = pre_trade_checks(addr, sym, quick=True)
        if rc["grade"] >= 3:
            push_feed({"sym_note": True, "msg": f"🛡️ {sym} RISK G{rc['grade']}: {', '.join(rc['reasons'][:2])}", "t": time.strftime("%H:%M:%S")})
            return
        if rc["grade"] == 2:
            push_feed({"sym_note": True, "msg": f"⚠️ {sym} caution: {', '.join(rc['cautions'][:2])}", "t": time.strftime("%H:%M:%S")})
        _rc_info = rc.get("raw", {}).get("info", {})
    except Exception as e:
        push_feed({"sym_note": True, "msg": f"⚠️ {sym} risk_check error: {e}", "t": time.strftime("%H:%M:%S")})
        # Non-fatal — proceed if risk_check fails

    # Quote
    sol_lamports = str(int(sol_amount * 1e9))
    try:
        quote = get_quote(C.SOL_ADDR, addr, sol_lamports, slippage)
        token_out = int(quote.get("toTokenAmount", 0))
        impact = float(quote.get("priceImpactPercent", quote.get("priceImpactPercentage", 100)))
        if token_out <= 0 or impact > 10:
            push_feed({"sym_note": True, "msg": f"⛔ {sym} bad quote", "t": time.strftime("%H:%M:%S")})
            return
    except Exception as e:
        push_feed({"sym_note": True, "msg": f"⛔ {sym} quote: {e}", "t": time.strftime("%H:%M:%S")})
        return

    # Build + Sign + Broadcast (Agentic Wallet TEE)
    if C.PAPER_TRADE:
        tx_hash = f"PAPER_{int(time.time())}"
        status = "SUCCESS"
    else:
        try:
            swap = swap_instruction(C.SOL_ADDR, addr, sol_lamports, slippage, WALLET_ADDRESS)
            tx_obj = swap.get("tx", "")
            unsigned_tx = tx_obj.get("data", "") if isinstance(tx_obj, dict) else tx_obj
            if not unsigned_tx:
                raise ValueError("Empty tx from swap")
            tx_to = tx_obj.get("to", addr) if isinstance(tx_obj, dict) else addr
            tx_hash = sign_and_broadcast(unsigned_tx, tx_to)
            if not tx_hash:
                raise ValueError("No txHash")
        except Exception as e:
            push_feed({"sym_note": True, "msg": f"❌ {sym} tx error: {e}", "t": time.strftime("%H:%M:%S")})
            return

        status = tx_status(tx_hash)
        if status == "FAILED":
            push_feed({"sym_note": True, "msg": f"❌ {sym} tx FAILED", "t": time.strftime("%H:%M:%S")})
            return

    # Balance verify
    _unconfirmed = False
    if not C.PAPER_TRADE:
        if status == "SUCCESS":
            time.sleep(2)
            actual = query_single_token_balance(addr)
            if actual > 0: token_out = actual
        elif status == "TIMEOUT":
            time.sleep(3)
            actual = query_single_token_balance(addr)
            if actual > 0: token_out = actual
            else: _unconfirmed = True

    # Record position
    tp1_p = entry_price * (1 + C.TP1_PCT)
    s1_pct = C.S1_PCT.get(tier) or C.S1_PCT.get(launch, -0.15)
    s1_p = entry_price * (1 + s1_pct)

    pos = {
        "symbol": sym, "address": addr, "tier": tier, "launch": launch,
        "entry": entry_price, "entry_mc": result.get("mc", 0),
        "entry_ts": time.time(), "entry_human": time.strftime("%m-%d %H:%M:%S"),
        "sol_in": sol_amount, "token_amount": token_out,
        "remaining": 1.0, "tp1_hit": False,
        "peak_price": entry_price,
        "s3a_warned": False, "sell_fails": 0, "stuck": False,
        "tp1": tp1_p, "s1": s1_p,
        "age_min": result.get("age_m", 0),
        "pnl_pct": 0.0, "current_price": entry_price,
        "confidence": conf,
        "near_migration": result.get("near_migration", False),
        "logo": fetch_token_logo(addr),
        "origin": "meme_trench_scanner",
        "entry_liquidity_usd": liq,
        "entry_top10": float(_rc_info.get("top10HoldPercent", 0) or 0),
        "entry_sniper_pct": float(_rc_info.get("sniperHoldingPercent", 0) or 0),
        "risk_last_checked": 0,
    }
    if _unconfirmed:
        pos["unconfirmed"] = True
        pos["unconfirmed_ts"] = time.time()
        pos["unconfirmed_checks"] = 0
    with pos_lock:
        positions[addr] = pos
        _pending_buys = max(0, _pending_buys - 1)
        _save_positions_unlocked()
    _buy_slot_reserved.flag = False
    sync_positions()

    with state_lock:
        state["stats"]["buys"] += 1

    push_feed({"sym_note": True,
               "msg": f"🛒 BUY ${sym} {tier}[{conf}] {sol_amount} SOL @ ${entry_price:.8f}",
               "t": time.strftime("%H:%M:%S")})
    reflect_on_entry(sym, tier, sol_amount, conf)


def try_open_position(result: dict):
    global _pending_buys
    sym = result.get("symbol", "?")
    try:
        _try_open_position_inner(result)
    except Exception as _e:
        import traceback
        push_feed({"sym_note": True, "msg": f"🔴 BUY CRASH [{sym}]: {_e}", "t": time.strftime("%H:%M:%S")})
        traceback.print_exc()
    finally:
        if getattr(_buy_slot_reserved, 'flag', False):
            with pos_lock:
                _pending_buys = max(0, _pending_buys - 1)
            _buy_slot_reserved.flag = False


# ── Sell Execution ──────────────────────────────────────────────────────

def close_position(addr: str, sell_pct: float, reason: str, current_price: float = 0, _mc_now: float = 0):
    with pos_lock:
        if addr not in positions: return
        if addr in _selling: return
        _selling.add(addr)
        pos = dict(positions[addr])

    try:
        if pos.get("stuck"):
            return

        sym = pos.get("symbol", addr[:8])
        # Query on-chain balance
        onchain_bal = query_single_token_balance(addr) if not C.PAPER_TRADE else pos.get("token_amount", 0)
        if onchain_bal <= 0:
            if onchain_bal == 0:
                if time.time() - pos.get("entry_ts", 0) < 30: return
                with pos_lock:
                    if addr in positions:
                        zbc = positions[addr].get("zero_balance_count", 0) + 1
                        positions[addr]["zero_balance_count"] = zbc
                        if zbc < 3:
                            _save_positions_unlocked()
                            return
                        positions.pop(addr, None)
                        _save_positions_unlocked()
                sync_positions()
                return
            else:
                onchain_bal = pos.get("token_amount", 0)
                if onchain_bal <= 0: return
        else:
            with pos_lock:
                if addr in positions and positions[addr].get("zero_balance_count", 0) > 0:
                    positions[addr]["zero_balance_count"] = 0

        sell_amount = int(onchain_bal * min(sell_pct, 1.0))
        if sell_amount <= 0: return

        # Execute sell
        if C.PAPER_TRADE:
            status = "SUCCESS"
        else:
            sell_fails = pos.get("sell_fails", 0)
            pnl_now = (current_price - pos["entry"]) / max(pos["entry"], 1e-18) * 100 if current_price > 0 else pos.get("pnl_pct", 0)
            if sell_fails >= 3 or pnl_now <= -40: dyn_slippage = 200
            elif sell_fails >= 1 or pnl_now <= -20: dyn_slippage = 100
            else: dyn_slippage = C.SLIPPAGE_SELL

            try:
                swap = swap_instruction(addr, C.SOL_ADDR, str(sell_amount), dyn_slippage, WALLET_ADDRESS)
                tx_obj = swap.get("tx", "")
                unsigned_tx = tx_obj.get("data", "") if isinstance(tx_obj, dict) else tx_obj
                if not unsigned_tx: raise ValueError("Empty tx (sell)")
                tx_to = tx_obj.get("to", C.SOL_ADDR) if isinstance(tx_obj, dict) else C.SOL_ADDR
                tx_hash = sign_and_broadcast(unsigned_tx, tx_to)
                if not tx_hash: raise ValueError("No txHash (sell)")
                status = tx_status(tx_hash)
            except Exception as e:
                push_feed({"sym_note": True, "msg": f"❌ SELL {sym}: {e}", "t": time.strftime("%H:%M:%S")})
                with pos_lock:
                    if addr in positions:
                        positions[addr]["sell_fails"] = positions[addr].get("sell_fails", 0) + 1
                        if positions[addr]["sell_fails"] >= 5:
                            positions[addr]["stuck"] = True
                    _save_positions_unlocked()
                return

            if status == "FAILED":
                with pos_lock:
                    if addr in positions:
                        positions[addr]["sell_fails"] = positions[addr].get("sell_fails", 0) + 1
                    _save_positions_unlocked()
                return

            if status == "TIMEOUT":
                time.sleep(3)
                post_bal = query_single_token_balance(addr)
                if post_bal < 0 or post_bal >= onchain_bal:
                    with pos_lock:
                        if addr in positions:
                            positions[addr]["sell_fails"] = positions[addr].get("sell_fails", 0) + 1
                        _save_positions_unlocked()
                    return

        # Post-sell leftover
        expected_leftover = onchain_bal - sell_amount
        is_partial = sell_pct < 0.99
        if C.PAPER_TRADE:
            leftover = expected_leftover if is_partial else 0
        else:
            if is_partial and expected_leftover > 0:
                time.sleep(3)
                rpc = query_single_token_balance(addr)
                leftover = rpc if rpc > 0 else expected_leftover
            else:
                time.sleep(3)
                leftover = query_single_token_balance(addr)
                if leftover < 0: leftover = max(0, expected_leftover)

        # PnL
        exit_mc = _mc_now
        if current_price > 0:
            exit_price = current_price
        else:
            try:
                pi = price_info(addr)
                exit_price = float(pi.get("price", pos["entry"]))
                if exit_mc <= 0: exit_mc = float(pi.get("marketCap", 0))
            except Exception:
                exit_price = pos["entry"]

        if pos["entry"] <= 0:
            gross_pct = 0.0
        else:
            gross_pct = (exit_price - pos["entry"]) / pos["entry"] * 100
        net_pct = gross_pct - C.COST_PER_LEG * 100 * 2  # Use config value instead of hardcoded
        sold_fraction = sell_amount / max(onchain_bal, 1)
        net_sol = pos["sol_in"] * pos["remaining"] * sold_fraction * (gross_pct / 100)

        if leftover <= 0:
            with pos_lock:
                positions.pop(addr, None)
                recently_closed[addr] = time.time()
                _save_positions_unlocked()
            save_recently_closed()
            sync_positions()

            trade = {
                "t": time.strftime("%m-%d %H:%M"), "symbol": sym, "tier": pos["tier"],
                "launch": pos["launch"], "entry_mc": pos["entry_mc"], "exit_mc": exit_mc,
                "pnl_pct": round(gross_pct, 2), "sol_in": pos["sol_in"],
                "pnl_sol": round(net_sol, 6), "reason": f"{reason} {gross_pct:+.1f}%",
                "stuck": False, "confidence": pos.get("confidence", 0),
            }
            with state_lock:
                state["trades"].insert(0, trade)
                state["stats"]["sells"] += 1
                state["stats"]["net_sol"] = round(state["stats"]["net_sol"] + net_sol, 6)
                if net_pct > 0: state["stats"]["wins"] += 1
                else: state["stats"]["losses"] += 1
                if net_pct > 0: state["stats"]["pos_wins"] = state["stats"].get("pos_wins", 0) + 1
                else: state["stats"]["pos_losses"] = state["stats"].get("pos_losses", 0) + 1
            save_trades()

            if net_pct < 0: record_loss(abs(net_sol))
            else: record_win()
            reflect_on_exit(sym, pos.get("tier", "SCALP"), net_sol, reason, (time.time() - pos["entry_ts"]) / 60)

            icon = "✅" if gross_pct > 0 else "❌"
            push_feed({"sym_note": True,
                       "msg": f"{icon} {reason}: ${sym} {gross_pct:+.1f}% {(time.time()-pos['entry_ts'])/60:.1f}min",
                       "t": time.strftime("%H:%M:%S")})
        else:
            new_remaining = round(pos["remaining"] * (leftover / max(onchain_bal, 1)), 3)
            with pos_lock:
                if addr in positions:
                    positions[addr]["token_amount"] = leftover
                    positions[addr]["remaining"] = max(new_remaining, 0.001)
                    positions[addr]["tp1_hit"] = True
                    positions[addr]["s1"] = positions[addr]["entry"]
                    positions[addr]["sell_fails"] = 0
                _save_positions_unlocked()
            sync_positions()

            trade = {
                "t": time.strftime("%m-%d %H:%M"), "symbol": sym, "tier": pos["tier"],
                "launch": pos["launch"], "entry_mc": pos["entry_mc"], "exit_mc": exit_mc,
                "pnl_pct": round(gross_pct, 2), "sol_in": round(pos["sol_in"] * sold_fraction, 4),
                "pnl_sol": round(net_sol, 6), "reason": f"{reason} {int(sold_fraction*100)}%",
                "stuck": False, "confidence": pos.get("confidence", 0), "partial": True,
            }
            with state_lock:
                state["trades"].insert(0, trade)
                state["stats"]["sells"] += 1
                state["stats"]["net_sol"] = round(state["stats"]["net_sol"] + net_sol, 6)
                if net_pct > 0: state["stats"]["wins"] += 1
                else: state["stats"]["losses"] += 1
            save_trades()

            push_feed({"sym_note": True,
                       "msg": f"✅ {reason}: ${sym} {gross_pct:+.1f}% sold {sold_fraction:.0%}",
                       "t": time.strftime("%H:%M:%S")})
    finally:
        with pos_lock: _selling.discard(addr)


# ── Position Monitor ────────────────────────────────────────────────────

def check_position(addr: str):
    with pos_lock:
        if addr not in positions: return
        pos = dict(positions[addr])

    if pos.get("stuck"): return

    # Unconfirmed verification
    if pos.get("unconfirmed"):
        elapsed = time.time() - pos.get("unconfirmed_ts", pos.get("entry_ts", 0))
        checks = pos.get("unconfirmed_checks", 0)
        if elapsed < 60: return
        bal = query_single_token_balance(addr)
        if bal > 0:
            with pos_lock:
                if addr in positions:
                    positions[addr].pop("unconfirmed", None)
                    positions[addr]["token_amount"] = bal
                _save_positions_unlocked()
            sync_positions()
            return
        elif bal == -1: return
        else:
            checks += 1
            with pos_lock:
                if addr in positions:
                    positions[addr]["unconfirmed_checks"] = checks
            if checks >= 10 and elapsed >= 180:
                with pos_lock:
                    positions.pop(addr, None)
                    _save_positions_unlocked()
                sync_positions()
            return

    try:
        pi = _price_cache.get(addr) or price_info(addr)
    except Exception: return

    price = float(pi.get("price", pos["entry"]))
    _mc_now = float(pi.get("marketCap", 0))
    entry_p = float(pos["entry"])
    if entry_p <= 0: return

    pct = (price - entry_p) / entry_p * 100
    elapsed = (time.time() - pos["entry_ts"]) / 60
    tier = pos["tier"]
    launch = pos.get("launch", "quiet")
    tp1_hit = pos["tp1_hit"]

    with pos_lock:
        if addr not in positions:
            return
        positions[addr]["peak_price"] = max(positions[addr].get("peak_price", price), price)
        positions[addr]["pnl_pct"] = round(pct, 2)
        positions[addr]["current_price"] = price
        _ph = positions[addr].setdefault("_price_hist", [])
        _ph.append((time.time(), price))
        _cutoff = time.time() - 30
        positions[addr]["_price_hist"] = [(t, p) for t, p in _ph if t > _cutoff]
        peak = positions[addr]["peak_price"]
    sync_positions()
    peak_pct = (peak - entry_p) / entry_p * 100

    # HE1
    if pct <= C.HE1_PCT * 100:
        close_position(addr, 1.0, "HE1", current_price=price, _mc_now=_mc_now); return

    # MaxHold
    if elapsed >= C.MAX_HOLD_MIN:
        close_position(addr, 1.0, f"MaxHold {elapsed:.0f}m", current_price=price, _mc_now=_mc_now); return

    # Fast dump
    if not tp1_hit:
        with pos_lock:
            _ph = positions.get(addr, {}).get("_price_hist", [])
        if len(_ph) >= 2:
            _now_t = time.time()
            _wp = [(t, p) for t, p in _ph if _now_t - t <= C.FAST_DUMP_SEC]
            if _wp:
                _wh = max(p for _, p in _wp)
                if _wh > 0 and (price - _wh) / _wh <= C.FAST_DUMP_PCT:
                    close_position(addr, 1.0, "FAST_DUMP", current_price=price, _mc_now=_mc_now); return

    # Trailing
    if tp1_hit and peak > entry_p:
        if price < peak * (1 - C.TRAILING_DROP):
            close_position(addr, 1.0, "Trailing", current_price=price, _mc_now=_mc_now); return

    # S1
    s1_price = pos["s1"]
    if price <= s1_price:
        label = "S1_BE" if tp1_hit else "S1_STOP"
        close_position(addr, 1.0, label, current_price=price, _mc_now=_mc_now); return

    # S3 time stops
    s3_key = launch  # tier is never SCALP; always key by launch type
    s3_limit = C.S3_MIN.get(s3_key, C.S3_MIN.get("quiet", 15))
    if elapsed >= s3_limit and pct < C.TP1_PCT * 100:
        close_position(addr, 1.0, "S3_TIME", current_price=price, _mc_now=_mc_now); return

    # TP2
    if tp1_hit and pct >= C.TP2_PCT * 100:
        close_position(addr, 1.0, "TP2", current_price=price, _mc_now=_mc_now); return

    # TP1
    if not tp1_hit and pct >= C.TP1_PCT * 100:
        tp1_sell = C.TP1_SELL.get(launch, 0.50)
        close_position(addr, tp1_sell, "TP1", current_price=price, _mc_now=_mc_now); return


def _quick_wallet_sync():
    """Sync token_amount for existing positions only. Never auto-adopt unknown wallet tokens."""
    try:
        onchain = query_all_wallet_tokens()
        if onchain is None: return
        onchain = {m: a for m, a in onchain.items() if m not in C._IGNORE_MINTS}
    except Exception: return
    if not onchain: return

    updated = False
    with pos_lock:
        for mint, amount in onchain.items():
            if mint in positions:
                positions[mint]["token_amount"] = amount
                updated = True
    if updated: sync_positions()


def wallet_audit():
    global _last_wallet_audit
    _last_wallet_audit = time.time()
    onchain = query_all_wallet_tokens()
    if onchain is None: return
    onchain = {m: a for m, a in onchain.items() if m not in C._IGNORE_MINTS}
    if not onchain and not positions: return
    with pos_lock:
        if not onchain and len(positions) > 0: return
        # Guard: if API returns far fewer tokens than we track, skip audit
        # to avoid false deletions from incomplete API responses
        tracked_in_onchain = sum(1 for a in positions if a in onchain)
        if len(positions) > 0 and tracked_in_onchain == 0 and len(onchain) > 0:
            return  # API likely returned incomplete data
    with pos_lock:
        for addr in list(positions.keys()):
            if addr not in onchain:
                miss = positions[addr].get("_audit_miss", 0) + 1
                positions[addr]["_audit_miss"] = miss
                if miss < 3: continue
                push_feed({"sym_note": True,
                           "msg": f"⚠️ Audit: {positions[addr].get('symbol', addr[:8])} removed — not found on-chain 3x",
                           "t": time.strftime("%H:%M:%S")})
                positions.pop(addr, None)
            else:
                if "_audit_miss" in positions[addr]: del positions[addr]["_audit_miss"]
                if positions[addr].get("zero_balance_count", 0) > 0:
                    positions[addr]["zero_balance_count"] = 0
        for addr in list(positions.keys()):
            if addr in onchain:
                positions[addr]["token_amount"] = onchain[addr]
        _save_positions_unlocked()
    sync_positions()


def monitor_loop():
    global _last_wallet_audit, _price_cache
    while True:
        try:
            _quick_wallet_sync()
            with pos_lock: addr_list = list(positions.keys())

            # Batch price fetch: 1 API call instead of N
            if addr_list:
                try:
                    tokens_param = ",".join(f"501:{a}" for a in addr_list)
                    batch = _onchainos("market", "prices", "--tokens", tokens_param, timeout=15)
                    batch_data = _cli_data(batch)
                    items = batch_data if isinstance(batch_data, list) else [batch_data] if isinstance(batch_data, dict) else []
                    _price_cache = {item.get("tokenContractAddress", item.get("tokenAddress", "")): item for item in items if isinstance(item, dict)}
                except Exception:
                    _price_cache = {}  # Fallback: check_position will query individually

            for addr in addr_list:
                try:
                    check_position(addr)
                except Exception as e:
                    push_feed({"sym_note": True, "msg": f"🔴 check_position: {e}", "t": time.strftime("%H:%M:%S")})
                try:
                    pnl = portfolio_token_pnl(addr)
                    if pnl:
                        with pos_lock:
                            if addr in positions:
                                positions[addr]["realized_pnl_usd"] = float(pnl.get("realizedPnlUsd", 0))
                                positions[addr]["unrealized_pnl_usd"] = float(pnl.get("unrealizedPnlUsd", 0))
                        sync_positions()
                except Exception: pass

                # Risk check post-trade monitoring (throttled to 60s per position)
                try:
                    with pos_lock:
                        if addr in positions:
                            _p = positions[addr]
                            _rlc = _p.get("risk_last_checked", 0)
                            _eliq = _p.get("entry_liquidity_usd", 0)
                            _et10 = _p.get("entry_top10", 0)
                            _esp = _p.get("entry_sniper_pct", 0)
                            _sym = _p.get("symbol", addr[:8])
                        else:
                            _rlc = time.time()  # skip
                            _eliq = _et10 = _esp = 0
                            _sym = ""
                    if time.time() - _rlc >= 60 and _sym:
                        with pos_lock:
                            if addr in positions:
                                positions[addr]["risk_last_checked"] = time.time()
                        def _run_risk_flags(_addr=addr, _sym=_sym, _eliq=_eliq, _et10=_et10, _esp=_esp):
                            try:
                                flags = post_trade_flags(_addr, _sym,
                                    entry_liquidity_usd=_eliq, entry_top10=_et10, entry_sniper_pct=_esp)
                                for flag in flags:
                                    push_feed({"sym_note": True, "msg": f"🛡️ {_sym} {flag}", "t": time.strftime("%H:%M:%S")})
                                    if flag.startswith("EXIT_NOW"):
                                        close_position(_addr, 1.0, f"RISK:{flag[:40]}", _mc_now=0)
                                        break
                            except Exception:
                                pass
                        threading.Thread(target=_run_risk_flags, daemon=True).start()
                except Exception:
                    pass

            if time.time() - _last_wallet_audit >= _WALLET_AUDIT_SEC:
                try:
                    wallet_audit()
                except Exception as e:
                    push_feed({"sym_note": True, "msg": f"⚠️ audit: {e}", "t": time.strftime("%H:%M:%S")})

            # Cleanup recently_closed
            now = time.time()
            expired = False
            with pos_lock:
                for addr in list(recently_closed.keys()):
                    if now - recently_closed[addr] > 7200:
                        del recently_closed[addr]
                        expired = True
            if expired: save_recently_closed()

            time.sleep(C.MONITOR_SEC)
        except Exception as e:
            push_feed({"sym_note": True, "msg": f"🔴 MONITOR: {e}", "t": time.strftime("%H:%M:%S")})
            time.sleep(C.MONITOR_SEC)


# ── Scanner Loop ────────────────────────────────────────────────────────

def scanner_loop():
    from concurrent.futures import ThreadPoolExecutor, as_completed
    cycle = 0
    while True:
        try:
            if state["session"]["stopped"]:
                time.sleep(60); continue

            cycle += 1
            with state_lock:
                state["cycle"] = cycle
                state["stats"]["cycles"] = cycle
                state["hot"] = state["session"].get("hot_mode", False)
                state["status"] = f"{'🌶️ HOT' if state['hot'] else '❄️'} #{cycle}"

            push_feed({"sep": True, "cycle": cycle, "hot": state["hot"], "t": time.strftime("%H:%M:%S")})

            try:
                migrated = memepump_token_list(protocol_ids=C.DISCOVERY_PROTOCOLS)
                try:
                    new_tokens = memepump_token_list(
                        stage="NEW", max_mc=C.MC_MAX_NEW, min_holders=10,
                        protocol_ids=C.DISCOVERY_PROTOCOLS, limit=30)
                except Exception: new_tokens = []
                seen = set()
                candidates = []
                for tok in migrated + new_tokens:
                    k = tok.get("tokenContractAddress", tok.get("tokenAddress", ""))
                    if k and k not in seen:
                        seen.add(k); candidates.append(tok)
            except Exception as e:
                push_feed({"sym_note": True, "msg": f"⚠️ memepump error: {e}", "t": time.strftime("%H:%M:%S")})
                try:
                    r5 = token_ranking(5); r2 = token_ranking(2)
                    seen = set(); candidates = []
                    for t in r5 + r2:
                        k = t.get("tokenContractAddress", t.get("tokenAddress", ""))
                        if k and k not in seen: seen.add(k); candidates.append(t)
                except Exception:
                    time.sleep(C.LOOP_SEC); continue

            hot_mode_check()
            survivors = pre_filter(candidates, time.time())

            results = []
            if survivors:
                with ThreadPoolExecutor(max_workers=min(len(survivors), 6)) as pool:
                    future_map = {pool.submit(detect_signal, tok): tok for tok in survivors}
                    for future in as_completed(future_map):
                        try:
                            results.append((future.result(), future_map[future]))
                        except Exception as e:
                            pass

            ACTIVE_TIERS = ("MINIMUM", "STRONG")
            for result, token in results:
                tier = result.get("tier", "NO_SIGNAL")
                push_feed({**result, "mc": result.get("mc", 0), "age_m": result.get("age_m", 0)})

                if tier in ACTIVE_TIERS:
                    mc_val = result.get("mc", 0)
                    sig_entry = {
                        **result, "mc": mc_val, "liq": 0,
                        "tp1_mc": round(mc_val * 1.15), "tp2_mc": round(mc_val * 1.25),
                        "s1_mc": round(mc_val * 0.85), "t": time.strftime("%H:%M:%S"),
                        "logo": fetch_token_logo(result.get("addr", "")),
                    }
                    with state_lock:
                        state["signals"].insert(0, sig_entry)
                        if len(state["signals"]) > 100:
                            state["signals"] = state["signals"][:100]

                    reflect_on_signal(result.get("symbol", "?"), tier, result.get("confidence", 0))
                    threading.Thread(target=try_open_position, args=(dict(result),), daemon=True).start()

            time.sleep(C.LOOP_SEC)
        except Exception as e:
            push_feed({"sym_note": True, "msg": f"🔴 SCANNER: {e}", "t": time.strftime("%H:%M:%S")})
            time.sleep(C.LOOP_SEC)


# ── Dashboard ───────────────────────────────────────────────────────────

_dashboard_html_path = PROJECT_DIR / "dashboard.html"

class DashHandler(BaseHTTPRequestHandler):
    def log_message(self, *a): pass

    def _json(self, obj):
        data = json.dumps(obj, ensure_ascii=False).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path in ("/", "/index.html"):
            if _dashboard_html_path.exists():
                html = _dashboard_html_path.read_text(encoding="utf-8")
            else:
                html = "<h1>dashboard.html not found</h1><p>Place dashboard.html in the same directory as scan_live.py</p>"
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            self.wfile.write(html.encode("utf-8"))
        elif self.path == "/api/state":
            with state_lock: snap = json.loads(json.dumps(state, ensure_ascii=False))
            snap["soul"] = soul_summary()
            # PnL curve from trade history
            curve = []
            running = 0.0
            for t in reversed(snap.get("trades", [])):
                sol_in = t.get("sol_in", 0)
                pnl_sol = t.get("pnl_sol", sol_in * (t.get("pnl_pct", 0) / 100))
                running = round(running + pnl_sol, 6)
                curve.append(running)
            snap["pnl_curve"] = curve
            self._json(snap)
        else:
            self.send_error(404)


def run_dashboard():
    port = C.DASHBOARD_PORT
    try:
        probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        probe.bind(("127.0.0.1", port))
        probe.close()
    except OSError:
        print(f"  ⚠️ Port {port} busy — terminating...")
        subprocess.run(f"lsof -ti:{port} | xargs kill -9", shell=True, capture_output=True)
        time.sleep(1.5)

    HTTPServer.allow_reuse_address = True
    server = HTTPServer(("127.0.0.1", port), DashHandler)
    server.serve_forever()


# ── Entry Point ─────────────────────────────────────────────────────────

if __name__ == "__main__":
    print("=" * 55)
    print("  Meme Trench Scanner v1.0 — Agentic Wallet TEE")
    print(f"  Wallet: {WALLET_ADDRESS[:8]}...{WALLET_ADDRESS[-4:]}" if not C.PAPER_TRADE else "  Mode: PAPER TRADE")
    print(f"  Dashboard: http://localhost:{C.DASHBOARD_PORT}")
    print(f"  Max: {C.MAX_SOL} SOL / {C.MAX_POSITIONS} positions")
    print(f"  PAUSED: {C.PAUSED}" + (" ← Set config.py PAUSED=False to start trading" if C.PAUSED else ""))
    print("=" * 55)

    load_on_startup()
    load_soul()

    threading.Thread(target=scanner_loop, daemon=True).start()
    threading.Thread(target=monitor_loop, daemon=True).start()

    print(f"  scanner_loop: every {C.LOOP_SEC}s")
    print(f"  monitor_loop: every {C.MONITOR_SEC}s")

    push_feed({"sym_note": True,
               "msg": f"🟢 Meme Trench Scanner started — {soul.get('name','')} [{soul.get('stage','')}] "
                      f"MC ${C.MC_MIN/1000:.0f}K-${C.MC_CAP/1000:.0f}K",
               "t": time.strftime("%H:%M:%S")})

    print(f"  → http://localhost:{C.DASHBOARD_PORT}")
    # Graceful shutdown handler
    def _shutdown_handler(signum, frame):
        print(f"\n  Received signal {signum}, shutting down...")
        with pos_lock:
            n = len(positions)
        if n > 0:
            print(f"  ⚠️ WARNING: {n} position(s) still open on-chain!")
            print(f"  Positions saved in {POSITIONS_FILE}, will resume on next start.")
        else:
            print("  No open positions.")
        print("  Done.")
        sys.exit(0)

    signal.signal(signal.SIGINT, _shutdown_handler)
    signal.signal(signal.SIGTERM, _shutdown_handler)

    try:
        run_dashboard()
    except KeyboardInterrupt:
        print("\n  Bot stopped.")
