"""
Top Rank Tokens Sniper v1.0 — 榜单狙击手
Dashboard: http://localhost:3244

Run: python3 ranking_sniper.py
Requires: onchainos CLI >= 2.0.0-beta (onchainos wallet login required)
No pip install needed for any third-party packages
"""

import os, sys, time, json, subprocess, shutil, threading, random, string
from pathlib import Path
from http.server import HTTPServer, BaseHTTPRequestHandler
from datetime import datetime, timezone

# ── Load Config ──────────────────────────────────────────────────────────
PROJECT_DIR = Path(__file__).parent
sys.path.insert(0, str(PROJECT_DIR))
import config as C
from risk_check import pre_trade_checks, post_trade_flags

STATE_DIR = PROJECT_DIR / "state"
WSOL = "So11111111111111111111111111111111111111112"
SOL_NATIVE = "11111111111111111111111111111111"

# ── onchainos CLI ───────────────────────────────────────────────────────

_ONCHAINOS = shutil.which("onchainos") or os.path.expanduser("~/.local/bin/onchainos")


def _check_onchainos():
    if not os.path.isfile(_ONCHAINOS):
        print("=" * 60)
        print("  FATAL: onchainos CLI not found")
        print(f"  Path: {_ONCHAINOS}")
        print("  Install: curl -fsSL https://onchainos.com/install.sh | bash")
        print("=" * 60)
        sys.exit(1)
    try:
        r = subprocess.run([_ONCHAINOS, "--version"], capture_output=True, text=True, timeout=10)
        print(f"  onchainos CLI: {r.stdout.strip()}")
    except Exception as e:
        print(f"  WARNING: onchainos --version failed: {e}")


def _onchainos(*args, timeout=30):
    cmd = [_ONCHAINOS] + list(args)
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        raise RuntimeError(f"onchainos timeout ({timeout}s): {' '.join(args[:3])}")
    out = result.stdout.strip()
    if not out:
        err = result.stderr.strip()
        raise RuntimeError(f"onchainos empty output (rc={result.returncode}): {err[:200]}")
    try:
        parsed = json.loads(out)
    except json.JSONDecodeError:
        raise RuntimeError(f"onchainos invalid JSON: {out[:200]}")
    if not parsed.get("ok", True):
        raise RuntimeError(f"onchainos error: {parsed.get('msg', out[:200])}")
    return parsed.get("data", parsed)


# ── Data API Layer ───────────────────────────────────────────────────────

def sf(v, fb=0):
    try:
        n = float(v)
        return n if n == n else fb  # NaN check
    except (TypeError, ValueError):
        return fb


def get_ranking(top_n=20):
    d = _onchainos("token", "trending", "--chain", "solana", "--sort-by", "2", "--time-frame", "2")
    return (d if isinstance(d, list) else [])[:top_n]


def get_advanced(addr):
    return _onchainos("token", "advanced-info", "--chain", "solana", "--address", addr)


def get_holders(addr, tag):
    d = _onchainos("token", "holders", "--chain", "solana", "--address", addr, "--tag-filter", str(tag))
    return d if isinstance(d, list) else []


def get_batch_prices(addrs):
    tokens = ",".join(f"501:{a}" for a in addrs)
    d = _onchainos("market", "prices", "--tokens", tokens)
    m = {}
    for i in (d if isinstance(d, list) else []):
        m[i.get("tokenContractAddress", "")] = sf(i.get("price"))
    return m


def get_sol_price():
    m = get_batch_prices([SOL_NATIVE])
    return m.get(SOL_NATIVE, 0)


def get_quote(from_, to_, amt):
    d = _onchainos("swap", "quote", "--from", from_, "--to", to_, "--amount", str(amt), "--chain", "solana")
    q = d[0] if isinstance(d, list) else d
    return {
        "routerResult": {
            "toTokenAmount": str(q.get("toTokenAmount", 0) if q else 0),
            "toTokenUsdPrice": (q.get("toToken", {}) or {}).get("tokenUnitPrice", "0") if q else "0",
            "toTokenDecimal": int((q.get("toToken", {}) or {}).get("decimal", 9)) if q else 9,
        }
    }


def get_swap(from_, to_, amt, wallet, slippage=2):
    d = _onchainos("swap", "swap", "--from", from_, "--to", to_, "--amount", str(amt),
                   "--chain", "solana", "--wallet", wallet, "--slippage", str(slippage))
    return d[0] if isinstance(d, list) else d


def get_wallet_tokens():
    d = _onchainos("wallet", "balance", "--chain", "501")
    assets = ((d or {}).get("details", [{}]) or [{}])[0].get("tokenAssets", [])
    return [a for a in assets if a.get("tokenAddress") and a["tokenAddress"] != "" and sf(a.get("balance")) > 0]


def wallet_addr():
    d = _onchainos("wallet", "addresses", "--chain", "501")
    addr = None
    if isinstance(d, dict):
        sol_list = d.get("solana", [])
        if sol_list:
            addr = sol_list[0].get("address")
        if not addr:
            addrs = d.get("addresses", [])
            if addrs:
                addr = addrs[0].get("address") if isinstance(addrs[0], dict) else addrs[0]
    elif isinstance(d, list) and d:
        addr = d[0].get("address") if isinstance(d[0], dict) else d[0]
    if not addr:
        raise RuntimeError("No Solana address — run: onchainos wallet login")
    return addr


def sol_balance():
    d = _onchainos("wallet", "balance", "--chain", "501")
    assets = ((d or {}).get("details", [{}]) or [{}])[0].get("tokenAssets", [])
    sol = next((a for a in assets if a.get("symbol") == "SOL" and a.get("tokenAddress", "") == ""), None)
    return sf(sol.get("balance")) if sol else 0


def sign_and_send(call_data, to):
    d = _onchainos("wallet", "contract-call", "--chain", "501", "--to", to, "--unsigned-tx", call_data)
    return {"success": True, "txHash": (d or {}).get("txHash", ""), "orderId": (d or {}).get("orderId", ""), "error": None}


def order_status(order_id):
    if not order_id:
        return "FAILED"
    try:
        # [C1] wallet order-status doesn't exist — use wallet history
        d = _onchainos("wallet", "history", "--tx-hash", order_id, "--chain-index", "501")
        item = d[0] if isinstance(d, list) and d else (d if isinstance(d, dict) else {})
        status = str(item.get("txStatus", "0"))
        if status in ("1", "2", "SUCCESS"):
            return "SUCCESS"
        if status in ("3", "FAILED"):
            return "FAILED"
        if status in ("TIMEOUT", "EXPIRED"):
            return "TIMEOUT"
        return "PENDING"
    except Exception:
        return "PENDING"


def query_token_balance(token_addr):
    try:
        d = _onchainos("wallet", "balance", "--chain", "501")
        assets = ((d or {}).get("details", [{}]) or [{}])[0].get("tokenAssets", [])
        tok = next((a for a in assets if a.get("tokenContractAddress") == token_addr or a.get("tokenAddress") == token_addr), None)
        return sf(tok.get("balance")) if tok else 0
    except Exception:
        return -1  # RPC error — caller must NOT treat as zero


# ── State Management ─────────────────────────────────────────────────────

_state_lock = threading.Lock()


def _ensure_dir(p):
    p.mkdir(parents=True, exist_ok=True)


def state_read(filename, fallback=None):
    fp = STATE_DIR / filename
    try:
        return json.loads(fp.read_text("utf-8"))
    except Exception:
        return fallback


def state_write(filename, data):
    fp = STATE_DIR / filename
    _ensure_dir(fp.parent)
    tmp = fp.with_suffix(fp.suffix + ".tmp")
    tmp.write_text(json.dumps(data, indent=2, ensure_ascii=False), "utf-8")
    tmp.rename(fp)


def _mode_file(f):
    return f"{C.MODE}/{f}"


def load_positions():
    return state_read(_mode_file("positions.json"), [])


def save_positions(p):
    state_write(_mode_file("positions.json"), p)


def load_trades():
    return state_read(_mode_file("trades.json"), [])


def add_trade(t):
    with _state_lock:
        a = load_trades()
        a.append(t)
        state_write(_mode_file("trades.json"), a)


def today_key():
    return datetime.now(timezone.utc).strftime("%Y-%m-%d")


def today_stats():
    all_stats = state_read(_mode_file("daily-stats.json"), {})
    k = today_key()
    if k not in all_stats:
        all_stats[k] = {"pnlSol": 0, "trades": 0, "wins": 0, "losses": 0}
        state_write(_mode_file("daily-stats.json"), all_stats)
    return all_stats[k]


def update_today(u):
    all_stats = state_read(_mode_file("daily-stats.json"), {})
    k = today_key()
    all_stats[k] = {**(all_stats.get(k) or {"pnlSol": 0, "trades": 0, "wins": 0, "losses": 0}), **u}
    state_write(_mode_file("daily-stats.json"), all_stats)


def add_signal(s):
    with _state_lock:
        a = state_read(_mode_file("signals-log.json"), [])
        a.append(s)
        if len(a) > 100:
            a = a[-100:]
        state_write(_mode_file("signals-log.json"), a)


# ── Engine State ────────────────────────────────────────────────────────

_engine_lock = threading.Lock()
_running = False
_prev_snap = set()
_first_poll = True
_cooldown = {}       # addr → timestamp
_buying = set()      # addresses currently being bought
_unconfirmed = {}    # addr → {pos, zero_count, start_time, order_id}
_roster = []         # current top N ranking
_logs = []           # engine logs
_MAX_LOG = 200
_poll_busy = False
_mon_busy = False
_audit_busy = False
_scanner_thread = None
_monitor_thread = None
_audit_thread = None
_wallet_cache = None   # cached Solana address — fetched once per engine start
_stop_event = threading.Event()

# Session risk control state
_session_risk = {
    "consecutive_losses": 0,
    "cumulative_loss_sol": 0.0,
    "paused_until": 0,
    "stopped": False,
}


def _record_session_loss(loss_sol):
    """Record loss, trigger session pause/stop"""
    _session_risk["consecutive_losses"] += 1
    _session_risk["cumulative_loss_sol"] += abs(loss_sol)
    if _session_risk["cumulative_loss_sol"] >= C.SESSION_STOP_SOL:
        _session_risk["stopped"] = True
        log("SESSION", f"🛑 STOPPED — cumulative loss {_session_risk['cumulative_loss_sol']:.4f} SOL >= {C.SESSION_STOP_SOL}")
    elif _session_risk["consecutive_losses"] >= C.MAX_CONSEC_LOSS:
        _session_risk["paused_until"] = time.time() + C.PAUSE_CONSEC_SEC
        log("SESSION", f"⏸ PAUSED {C.PAUSE_CONSEC_SEC//60}min — {_session_risk['consecutive_losses']} consecutive losses")


def _record_session_win():
    """Record win, reset consecutive loss counter"""
    _session_risk["consecutive_losses"] = 0


def log(type_, msg):
    ts = int(time.time() * 1000)
    entry = {"ts": ts, "type": type_, "msg": msg}
    with _engine_lock:
        _logs.append(entry)
        if len(_logs) > _MAX_LOG:
            _logs.pop(0)
    t_str = datetime.fromtimestamp(ts / 1000).strftime("%H:%M:%S")
    print(f"[{t_str}][{type_}] {msg}")


def engine_state():
    return {
        "running": _running,
        "mode": C.MODE,
        "version": "1.0.0",
        "positionsCount": len(load_positions()),
        "maxPositions": C.MAX_POSITIONS,
        "totalBudget": C.TOTAL_BUDGET,
    }


def get_logs(n=50):
    with _engine_lock:
        return list(_logs[-n:])


def get_roster():
    return list(_roster)


# ── Engine Start / Stop ─────────────────────────────────────────────────

def engine_start():
    global _running, _first_poll, _prev_snap, _scanner_thread, _monitor_thread, _audit_thread, _wallet_cache
    if _running:
        return {"ok": False, "msg": "Already running"}

    if C.MODE == "live":
        log("ENGINE", "Live mode — agentic wallet (onchainos wallet)")
        try:
            _wallet_cache = wallet_addr()   # cache once; avoids CLI call on every buy/sell
        except Exception as e:
            log("FATAL", f"Wallet connection failed: {e}")
            log("FATAL", "Please confirm: onchainos wallet login <email> has been executed")
            return {"ok": False, "msg": f"Wallet error: {e}"}
        try:
            _wallet_audit()
        except Exception as e:
            log("WARN", f"Wallet audit skipped: {e}")

    _running = True
    _first_poll = True
    _prev_snap = set()
    _cooldown.clear()
    _buying.clear()
    _unconfirmed.clear()
    _stop_event.clear()

    log("ENGINE", f"Started v1.0 | mode={C.MODE} | budget={C.TOTAL_BUDGET}SOL | per_trade={C.BUY_AMOUNT}SOL | max_pos={C.MAX_POSITIONS}")

    _scanner_thread = threading.Thread(target=_scanner_loop, daemon=True)
    _monitor_thread = threading.Thread(target=_monitor_loop, daemon=True)
    _scanner_thread.start()
    _monitor_thread.start()

    if C.MODE == "live":
        _audit_thread = threading.Thread(target=_audit_loop, daemon=True)
        _audit_thread.start()

    return {"ok": True, "msg": "Engine started"}


def engine_stop():
    global _running, _wallet_cache
    if not _running:
        return {"ok": False, "msg": "Not running"}
    _wallet_cache = None

    _running = False
    _stop_event.set()

    # Close all positions
    pos = load_positions()
    if pos:
        log("ENGINE", f"Closing {len(pos)} position(s)...")
        try:
            sp = get_sol_price()
        except Exception:
            sp = 0
        try:
            pm = get_batch_prices([p["tokenAddress"] for p in pos])
        except Exception:
            pm = {}
        failed = []
        for p in pos:
            try:
                cp = pm.get(p["tokenAddress"], sf(p.get("lastCheckPrice")))
                bp = sf(p.get("buyPrice"))
                pnl = ((cp - bp) / bp) * 100 if bp > 0 else 0
                _sell(p, 1, "StopExit", pnl, sp)
                log("SELL", f"{p['tokenSymbol']} | StopExit | PnL:{pnl:.1f}%")
            except Exception as e:
                log("ERROR", f"StopExit {p['tokenSymbol']}: {e}")
                failed.append(p)
        save_positions(failed)

    log("ENGINE", "Stopped")
    return {"ok": True, "msg": "Engine stopped"}


# ── Scanner Loop ────────────────────────────────────────────────────────

def _scanner_loop():
    while not _stop_event.is_set():
        if _running:
            _poll()
        _stop_event.wait(C.POLL_INTERVAL)


def _poll():
    global _poll_busy, _first_poll, _prev_snap, _roster
    if not _running or _poll_busy:
        return
    _poll_busy = True
    try:
        rank = get_ranking(C.TOP_N)
        if not rank:
            log("WARN", "Empty ranking")
            return
        _roster = rank
        cur = set(t.get("tokenContractAddress", "") for t in rank)

        if _first_poll:
            _prev_snap = cur
            _first_poll = False
            log("ENGINE", f"Initial snapshot: {len(rank)} tokens")
            return

        news = [t for t in rank if t.get("tokenContractAddress", "") not in _prev_snap]
        _prev_snap = cur
        if not news:
            return

        log("ENGINE", f"New entries: {', '.join(t.get('tokenSymbol', '?') for t in news)}")

        cands = []
        for t in news:
            r = _filter(t)
            if r:
                cands.append(r)

        cands.sort(key=lambda x: x["score"], reverse=True)

        delay = 2.0 if C.MODE == "live" else 0.3
        for i, cand in enumerate(cands):
            _buy(cand)
            if i < len(cands) - 1:
                time.sleep(delay)

    except Exception as e:
        log("ERROR", f"poll: {e}")
    finally:
        _poll_busy = False


# ── 3-Level Filter ──────────────────────────────────────────────────────

def _filter(tok):
    addr = tok.get("tokenContractAddress", "")
    sym = tok.get("tokenSymbol", "?")
    ch = sf(tok.get("change"))
    liq = sf(tok.get("liquidity"))
    mc = sf(tok.get("marketCap"))
    hold = sf(tok.get("holders"))
    txs = sf(tok.get("txs"), 1)
    txs_buy = sf(tok.get("txsBuy"))
    tr = sf(tok.get("uniqueTraders"))
    br = txs_buy / txs if txs > 0 else 0

    # Level 1: Slot Guard
    rej = []
    if ch < C.MIN_CHANGE_PCT:
        rej.append(f"change<{C.MIN_CHANGE_PCT}%")
    if ch > C.MAX_CHANGE_PCT:
        rej.append(f"change>{C.MAX_CHANGE_PCT}%")
    if liq < C.MIN_LIQUIDITY:
        rej.append(f"liq<${C.MIN_LIQUIDITY}")
    if mc < C.MIN_MCAP:
        rej.append(f"mcap<${C.MIN_MCAP}")
    if mc > C.MAX_MCAP:
        rej.append(f"mcap>${C.MAX_MCAP}")
    if hold < C.MIN_HOLDERS:
        rej.append(f"holders<{C.MIN_HOLDERS}")
    if br < C.MIN_BUY_RATIO:
        rej.append(f"buyRatio<{C.MIN_BUY_RATIO * 100:.0f}%")
    if tr < C.MIN_TRADERS:
        rej.append(f"traders<{C.MIN_TRADERS}")
    if addr in set(C.SKIP_TOKENS) | set(C.BLACKLIST):
        rej.append("blacklisted")

    ls = _cooldown.get(addr)
    if ls and time.time() * 1000 - ls < C.COOLDOWN_MIN * 60000:
        rej.append("cooldown")

    pos = load_positions()
    if len(pos) >= C.MAX_POSITIONS:
        rej.append("max_positions")
    if any(p["tokenAddress"] == addr for p in pos):
        rej.append("already_held")

    td = today_stats()
    if td["pnlSol"] < 0 and abs(td["pnlSol"]) >= C.TOTAL_BUDGET * C.DAILY_LOSS_LIMIT:
        rej.append("daily_loss_limit")

    # Session risk control check
    if _session_risk["stopped"]:
        rej.append("session_stopped")
    elif _session_risk["paused_until"] > time.time():
        remain = int((_session_risk["paused_until"] - time.time()) / 60)
        rej.append(f"session_paused_{remain}min")

    if rej:
        log("SKIP", f"{sym}: {', '.join(rej)}")
        add_signal({"ts": int(time.time() * 1000), "token": sym, "addr": addr, "type": "SKIP", "reasons": rej})
        return None

    # Level 2: Advanced Safety Check
    try:
        adv = get_advanced(addr)
    except Exception as e:
        log("SAFETY_REJECT", f"{sym}: api_error: {e}")
        add_signal({"ts": int(time.time() * 1000), "token": sym, "addr": addr, "type": "SAFETY_REJECT", "reasons": ["api_error"]})
        return None

    sr = []
    rl = sf((adv or {}).get("riskControlLevel"), 3)
    t10 = sf((adv or {}).get("top10HoldPercent"), 100)
    dh = sf((adv or {}).get("devHoldingPercent"), 100)
    bh = sf((adv or {}).get("bundleHoldingPercent"), 100)
    lpb = sf((adv or {}).get("lpBurnedPercent"), 0)
    drc = sf((adv or {}).get("devRugPullTokenCount"), 999)
    dev_created = sf((adv or {}).get("devCreateTokenCount", (adv or {}).get("devLaunchedTokenCount", 0)), 0)
    snh = sf((adv or {}).get("sniperHoldingPercent"), 100)
    is_int = (adv or {}).get("isInternal")

    raw_tags = (adv or {}).get("tokenTags", [])
    if isinstance(raw_tags, str):
        tags = raw_tags.split(",")
    elif isinstance(raw_tags, list):
        tags = raw_tags
    else:
        tags = []

    if rl > C.MAX_RISK_LEVEL:
        sr.append(f"RiskLevel:{rl}")
    if C.BLOCK_HONEYPOT and any("honeypot" in (t if isinstance(t, str) else "").lower() for t in tags):
        sr.append("Honeypot")
    if t10 > C.MAX_TOP10_HOLD:
        sr.append(f"Top10:{t10:.1f}%")
    if dh > C.MAX_DEV_HOLD:
        sr.append(f"DevHold:{dh:.1f}%")
    if bh > C.MAX_BUNDLE_HOLD:
        sr.append(f"Bundle:{bh:.1f}%")
    if not is_int and lpb < C.MIN_LP_BURN:
        sr.append(f"LPBurn:{lpb:.1f}%")
    # Rate-based rug check (aligned with risk_check.py)
    rug_rate = drc / max(dev_created, 1) if dev_created > 0 else (1.0 if drc > 0 else 0.0)
    if rug_rate >= 0.20 and drc >= 3:
        sr.append(f"SerialRugger:rate={rug_rate*100:.0f}%×{drc:.0f}")
    elif drc > C.MAX_DEV_RUG_COUNT:
        sr.append(f"DevRug:{drc:.0f}")
    if snh > C.MAX_SNIPER_HOLD:
        sr.append(f"Sniper:{snh:.1f}%")
    if C.BLOCK_INTERNAL and is_int is True:
        sr.append("Internal")

    if sr:
        log("SAFETY_REJECT", f"{sym}: {', '.join(sr)}")
        add_signal({"ts": int(time.time() * 1000), "token": sym, "addr": addr, "type": "SAFETY_REJECT", "reasons": sr})
        return None

    # Level 3: Holder Risk Scan
    try:
        sus_d = get_holders(addr, 6)
        phi_d = get_holders(addr, 8)
    except Exception as e:
        log("HOLDER_REJECT", f"{sym}: api_error: {e}")
        add_signal({"ts": int(time.time() * 1000), "token": sym, "addr": addr, "type": "HOLDER_REJECT", "reasons": ["api_error"]})
        return None

    hr = []
    sus_act = [h for h in sus_d if sf(h.get("holdPercent")) > 0]
    sus_p = sum(sf(h.get("holdPercent")) * 100 for h in sus_act)
    phi_act = [h for h in phi_d if sf(h.get("holdPercent")) > 0]

    if sus_p > C.MAX_SUSPICIOUS_HOLD:
        hr.append(f"SuspiciousHold:{sus_p:.1f}%")
    if C.BLOCK_PHISHING and len(phi_act) > 0:
        hr.append(f"PhishingHolder:{len(phi_act)}")
    if len(sus_act) > C.MAX_SUSPICIOUS_COUNT:
        hr.append(f"SuspiciousCount:{len(sus_act)}")

    if hr:
        log("HOLDER_REJECT", f"{sym}: {', '.join(hr)}")
        add_signal({"ts": int(time.time() * 1000), "token": sym, "addr": addr, "type": "HOLDER_REJECT", "reasons": hr})
        return None

    # Momentum Score
    score = _calc_score(tok, adv, tags, len(sus_act))
    log("PASS", f"{sym} | +{ch:.1f}% | BR:{br * 100:.0f}% | Score:{score}")
    add_signal({"ts": int(time.time() * 1000), "token": sym, "addr": addr, "type": "PASS", "score": score, "change": ch})
    return {"tok": tok, "adv": adv, "tags": tags, "score": score, "ch": ch, "br": br, "sus_c": len(sus_act)}


def _calc_score(tok, adv, tags, sus_c):
    ch = sf(tok.get("change"))
    txs = sf(tok.get("txs"), 1)
    br = sf(tok.get("txsBuy")) / txs if txs > 0 else 0
    tr = sf(tok.get("uniqueTraders"))
    liq = sf(tok.get("liquidity"))

    base = (min(br, 1) * 40
            + (max(0, 20 - (ch - 100) / 10) if ch > 100 else min(ch / 5, 20))
            + min(tr / 50, 1) * 20
            + min(liq / 50000, 1) * 20)

    tl = [(t.lower() if isinstance(t, str) else "") for t in tags]
    b = 0
    if any("smartmoneybuy" in t for t in tl):
        b += 8
    t10 = sf((adv or {}).get("top10HoldPercent"), 100)
    if t10 < 30:
        b += 5
    elif t10 < 50:
        b += 2
    if any("dspaid" in t for t in tl):
        b += 3
    if any("communitytakeover" in t for t in tl):
        b += 2
    sn = sf((adv or {}).get("sniperHoldingPercent"), 100)
    if sn < 5:
        b += 4
    elif sn < 10:
        b += 2
    if sf((adv or {}).get("devHoldingPercent"), 100) == 0 and sf((adv or {}).get("devRugPullTokenCount"), 999) < 3:
        b += 3
    if sus_c == 0:
        b += 2

    return round(base + min(b, 25))


# ── Buy ─────────────────────────────────────────────────────────────────

def _buy(cand):
    tok = cand["tok"]
    adv = cand["adv"]
    tags = cand["tags"]
    score = cand["score"]
    ch = cand["ch"]
    br = cand["br"]
    addr = tok.get("tokenContractAddress", "")
    sym = tok.get("tokenSymbol", "?")
    dec = int(sf(tok.get("decimal"), 9))

    if addr in _buying:
        return
    _buying.add(addr)

    try:
        amt = C.BUY_AMOUNT
        if len(load_positions()) >= C.MAX_POSITIONS:
            log("SKIP", f"{sym}: max_positions")
            return

        # Risk check — honeypot, wash trading, rug rate
        try:
            rc = pre_trade_checks(addr, sym, quick=True)
            if rc["grade"] >= 3:
                log("RISK_BLOCK", f"{sym}: G{rc['grade']} — {', '.join(rc['reasons'][:2])}")
                add_signal({"ts": int(time.time() * 1000), "token": sym, "addr": addr, "type": "RISK_BLOCK", "reasons": rc["reasons"]})
                return
            if rc["grade"] == 2:
                log("RISK_CAUTION", f"{sym}: {', '.join(rc['cautions'][:2])}")
        except Exception as e:
            log("WARN", f"{sym}: risk_check error: {e}")
            # Non-fatal — proceed if risk_check fails
        _rc_info = rc.get("raw", {}).get("info", {}) if 'rc' in locals() else {}
        _rc_liq = rc.get("raw", {}).get("liquidity_usd", 0) if 'rc' in locals() else 0

        price = 0
        hold = 0.0
        tx_hash = ""

        if C.MODE == "paper":
            # Paper mode
            try:
                q = get_quote(SOL_NATIVE, addr, str(round(amt * 1e9)))
                rr = q.get("routerResult", {})
                dec = int(rr.get("toTokenDecimal", dec))
                hold = sf(rr.get("toTokenAmount")) / (10 ** dec)
                price = sf(rr.get("toTokenUsdPrice"))
                if not price:
                    price = sf(tok.get("price"))
                if not price and hold > 0:
                    sp = get_sol_price()
                    price = (amt * sp) / hold
            except Exception:
                price = sf(tok.get("price"))
                try:
                    sp = get_sol_price()
                    if price > 0 and sp > 0:
                        hold = (amt * sp) / price
                except Exception:
                    pass

            if not price or price <= 0:
                log("SKIP", f"{sym}: price=0")
                return

            with _state_lock:
                pos = load_positions()
                pos.append(_make_position(addr, sym, dec, price, amt, hold, ch, score, {}))
                # Attach risk_check snapshots
                if 'rc' in locals() and rc.get("raw"):
                    pos[-1]["entry_liquidity_usd"] = rc["raw"].get("liquidity_usd", 0)
                    pos[-1]["entry_top10"] = float(rc["raw"].get("info", {}).get("top10HoldPercent", 0) or 0)
                    pos[-1]["entry_sniper_pct"] = float(rc["raw"].get("info", {}).get("sniperHoldingPercent", 0) or 0)
                save_positions(pos)
            add_trade(_make_trade("buy", addr, sym, amt, hold, price, tx_hash, f"rank_score_{score}", "0", "0"))
            log("BUY", f"{sym} | +{sf(ch):.0f}% | BR:{sf(br) * 100:.0f}% | S:{score} | {amt}SOL | ${price}")

        else:
            # Live mode
            try:
                bal = sol_balance()
                min_required = amt + C.GAS_RESERVE
                if bal < min_required:
                    log("SKIP", f"{sym}: balance {bal:.4f} < {min_required:.4f} (buy {amt} + gas {C.GAS_RESERVE})")
                    return

                w_addr = _wallet_cache or wallet_addr()
                log("ENGINE", f"{sym}: getSwap...")
                sw = get_swap(SOL_NATIVE, addr, str(round(amt * 1e9)), w_addr, C.SLIPPAGE_BUY)

                tx_data = (sw or {}).get("tx", {})
                if not tx_data.get("data"):
                    raise RuntimeError(f"No callData: {json.dumps(sw)[:300]}")

                log("ENGINE", f"{sym}: signAndSend...")
                res = sign_and_send(tx_data["data"], tx_data["to"])
                if not res["success"]:
                    log("ERROR", f"{sym}: tx fail: {res['error']}")
                    return
                tx_hash = res["txHash"]
                o_id = res["orderId"]

                # Layer 1: order_status confirmation
                tx_status = "PENDING"
                if o_id:
                    time.sleep(2)
                    for _ in range(5):
                        tx_status = order_status(o_id)
                        if tx_status != "PENDING":
                            break
                        time.sleep(2)
                elif tx_hash:
                    tx_status = "SUCCESS"

                if tx_status == "FAILED":
                    log("ERROR", f"{sym}: tx FAILED on-chain (orderId: {o_id})")
                    return

                # Extract swap result
                rr = (sw or {}).get("routerResult", {})
                dec = int(sf(rr.get("toToken", {}).get("decimal", rr.get("toTokenDecimal", dec))))
                hold = sf(rr.get("toTokenAmount")) / (10 ** dec)
                price = sf(rr.get("toTokenUsdPrice")) or sf(tok.get("price"))

                # Layer 2: on-chain balance verification
                confirmed = False
                if tx_status == "SUCCESS":
                    time.sleep(1)
                    on_chain_bal = query_token_balance(addr)
                    if on_chain_bal > 0:
                        confirmed = True
                        hold = on_chain_bal
                        log("LIVE_BUY", f"{sym} | tx: {tx_hash} | ${price} | balance verified: {hold}")
                    elif on_chain_bal == -1:
                        confirmed = True
                        log("LIVE_BUY", f"{sym} | tx: {tx_hash} | ${price} | RPC error on verify, assuming success")
                    else:
                        log("WARN", f"{sym}: order SUCCESS but balance=0, marking unconfirmed")

                safety_d = {
                    "riskControlLevel": str((adv or {}).get("riskControlLevel", "")),
                    "top10HoldPercent": str((adv or {}).get("top10HoldPercent", "")),
                    "devHoldingPercent": str((adv or {}).get("devHoldingPercent", "")),
                    "sniperHoldingPercent": str((adv or {}).get("sniperHoldingPercent", "")),
                    "bundleHoldingPercent": str((adv or {}).get("bundleHoldingPercent", "")),
                    "devRugPullTokenCount": str((adv or {}).get("devRugPullTokenCount", "")),
                    "hasSmartMoney": any("smartmoneybuy" in (t.lower() if isinstance(t, str) else "") for t in tags),
                }

                if confirmed:
                    if not price or price <= 0:
                        log("SKIP", f"{sym}: price=0 after verification")
                        return
                    with _state_lock:
                        pos = load_positions()
                        pos.append(_make_position(addr, sym, dec, price, amt, hold, ch, score, safety_d))
                        # Attach risk_check snapshots
                        if 'rc' in locals() and rc.get("raw"):
                            pos[-1]["entry_liquidity_usd"] = rc["raw"].get("liquidity_usd", 0)
                            pos[-1]["entry_top10"] = float(rc["raw"].get("info", {}).get("top10HoldPercent", 0) or 0)
                            pos[-1]["entry_sniper_pct"] = float(rc["raw"].get("info", {}).get("sniperHoldingPercent", 0) or 0)
                        save_positions(pos)
                    add_trade(_make_trade("buy", addr, sym, amt, hold, price, tx_hash, f"rank_score_{score}", "0", "0"))
                    log("BUY", f"{sym} | +{sf(ch):.0f}% | BR:{sf(br) * 100:.0f}% | S:{score} | {amt}SOL | ${price} | balance verified")
                    return

                # Layer 3: unconfirmed position
                unconf_pos = _make_position(addr, sym, dec, price or sf(tok.get("price")), amt, hold, ch, score, safety_d)
                unconf_pos["unconfirmed"] = True
                unconf_pos["triggerReason"] += " (unconfirmed)"
                _unconfirmed[addr] = {"pos": unconf_pos, "zero_count": 0, "start_time": time.time() * 1000, "order_id": o_id}
                add_trade(_make_trade("buy", addr, sym, amt, hold, price, tx_hash, f"rank_score_{score}(unconfirmed)", "0", "0"))
                log("BUY_UNCONFIRMED", f"{sym} | tx: {tx_hash} | orderId: {o_id} | monitoring balance...")

            except Exception as e:
                log("ERROR", f"{sym}: live buy: {e}")

    except Exception as e:
        log("ERROR", f"buy {sym}: {e}")
    finally:
        _buying.discard(addr)


def _make_position(addr, sym, dec, price, amt, hold, ch, score, safety_d):
    now = int(time.time() * 1000)
    return {
        "tokenAddress": addr, "tokenSymbol": sym, "decimal": dec,
        "buyPrice": str(price), "buyAmountSol": str(amt), "holdAmount": str(hold),
        "buyCount": 1, "buyTimestamp": now,
        "lastCheckPrice": str(price), "lastCheckTime": now,
        "peakPrice": str(price), "takeProfitTier": 0,
        "triggerReason": f"Rank +{sf(ch):.0f}% S:{score}",
        "safetyData": safety_d,
        "entry_liquidity_usd": 0,
        "entry_top10": 0,
        "entry_sniper_pct": 0,
        "risk_last_checked": 0,
    }


def _make_trade(direction, addr, sym, amt_sol, amt_token, price, tx_hash, reason, pnl_pct, pnl_sol):
    return {
        "tradeId": f"{direction}-{int(time.time() * 1000)}-{addr[:4]}-{''.join(random.choices(string.ascii_lowercase, k=4))}",
        "timestamp": int(time.time() * 1000),
        "direction": direction,
        "tokenAddress": addr, "tokenSymbol": sym,
        "amountSol": str(amt_sol), "amountToken": str(amt_token),
        "priceUsd": str(price), "txHash": tx_hash,
        "reason": reason, "pnlPercent": str(pnl_pct), "pnlSol": str(pnl_sol),
        "mode": C.MODE,
    }


# ── Monitor Loop (6-layer exit) ────────────────────────────────────────

def _monitor_loop():
    while not _stop_event.is_set():
        if _running:
            _monitor()
        _stop_event.wait(C.MONITOR_INTERVAL)


def _monitor():
    global _mon_busy
    if not _running or _mon_busy:
        return
    _mon_busy = True
    try:
        # Layer 3: check unconfirmed positions
        if C.MODE == "live":
            _check_unconfirmed()

        pos = load_positions()
        if not pos:
            return

        try:
            sp = get_sol_price()
        except Exception:
            sp = 130

        try:
            pm = get_batch_prices([p["tokenAddress"] for p in pos])
        except Exception as e:
            log("WARN", f"price fetch: {e}")
            return

        rm = []
        for p in pos:
            try:
                cp = pm.get(p["tokenAddress"], 0)
                if cp <= 0:
                    continue
                p["lastCheckPrice"] = str(cp)
                p["lastCheckTime"] = int(time.time() * 1000)
                pk = sf(p.get("peakPrice"))
                if cp > pk:
                    p["peakPrice"] = str(cp)
                bp = sf(p.get("buyPrice"))
                if bp <= 0:
                    continue
                pnl = ((cp - bp) / bp) * 100
                mins = (time.time() * 1000 - p["buyTimestamp"]) / 60000

                # EXIT 0: Ranking Exit
                if C.ENABLE_RANKING_EXIT and mins >= 1 and not any(t.get("tokenContractAddress") == p["tokenAddress"] for t in _roster):
                    log("RANK_EXIT", f"{p['tokenSymbol']} dropped, PnL: {pnl:.1f}%")
                    _sell(p, 1, "RankExit", pnl, sp)
                    rm.append(p["tokenAddress"])
                    continue

                # EXIT 1: Hard Stop
                if pnl <= C.STOP_LOSS_PCT:
                    log("SELL", f"{p['tokenSymbol']} | HardSL | PnL:{pnl:.1f}%")
                    _sell(p, 1, f"SL({C.STOP_LOSS_PCT}%)", pnl, sp)
                    rm.append(p["tokenAddress"])
                    continue

                # EXIT 2: Quick Stop
                if mins >= C.QUICK_STOP_MIN and pnl <= C.QUICK_STOP_PCT:
                    log("SELL", f"{p['tokenSymbol']} | QuickSL | PnL:{pnl:.1f}%")
                    _sell(p, 1, "QuickSL", pnl, sp)
                    rm.append(p["tokenAddress"])
                    continue

                # EXIT 3: Trailing Stop
                ppnl = ((sf(p.get("peakPrice")) - bp) / bp) * 100
                if ppnl >= C.TRAILING_ACTIVATE and ppnl - pnl >= C.TRAILING_DROP:
                    log("SELL", f"{p['tokenSymbol']} | TrailSL | PnL:{pnl:.1f}%")
                    _sell(p, 1, "TrailSL", pnl, sp)
                    rm.append(p["tokenAddress"])
                    continue

                # EXIT 4: Time Stop
                if mins / 60 >= C.MAX_HOLD_HOURS:
                    log("SELL", f"{p['tokenSymbol']} | TimeSL | PnL:{pnl:.1f}%")
                    _sell(p, 1, "TimeSL", pnl, sp)
                    rm.append(p["tokenAddress"])
                    continue

                # EXIT 5: Tiered Take Profit
                ct = p.get("takeProfitTier", 0)
                for i in range(ct, len(C.TP_TIERS)):
                    tp_pct, tp_sell = C.TP_TIERS[i]
                    if pnl >= tp_pct:
                        log("SELL", f"{p['tokenSymbol']} | TP{i + 1}(+{tp_pct}%) | PnL:{pnl:.1f}%")
                        prev_hold = p["holdAmount"]
                        try:
                            _sell(p, tp_sell, f"TP{i + 1}", pnl, sp)
                            p["takeProfitTier"] = i + 1
                            if i == len(C.TP_TIERS) - 1:
                                rm.append(p["tokenAddress"])
                        except Exception as e:
                            p["holdAmount"] = prev_hold
                            log("ERROR", f"TP sell {p['tokenSymbol']}: {e}")
                        break

            except Exception as e:
                log("ERROR", f"mon {p['tokenSymbol']}: {e}")

        if rm:
            save_positions([p for p in pos if p["tokenAddress"] not in rm])
        else:
            save_positions(pos)

        # Risk check post-trade monitoring (background, throttled 60s per position)
        for p in (load_positions() if not rm else [px for px in pos if px["tokenAddress"] not in rm]):
            _rlc = p.get("risk_last_checked", 0)
            if time.time() - _rlc < 60:
                continue
            # Update timestamp
            p["risk_last_checked"] = time.time()
            _addr = p["tokenAddress"]
            _sym = p["tokenSymbol"]
            _eliq = p.get("entry_liquidity_usd", 0)
            _et10 = p.get("entry_top10", 0)
            _esp = p.get("entry_sniper_pct", 0)
            def _run_rc(_a=_addr, _s=_sym, _el=_eliq, _t10=_et10, _sp=_esp):
                try:
                    flags = post_trade_flags(_a, _s, entry_liquidity_usd=_el, entry_top10=_t10, entry_sniper_pct=_sp)
                    for flag in flags:
                        log("RISK_FLAG", f"{_s}: {flag}")
                        if flag.startswith("EXIT_NOW"):
                            log("RISK_EXIT", f"{_s}: {flag}")
                            # Actually close the position — get current price for PnL
                            try:
                                _pi = price_info(_a)
                                _cp = sf(_pi.get("price"))
                            except Exception:
                                _cp = 0
                            _sell_pos = None
                            with _state_lock:
                                _all = load_positions()
                                _sell_pos = next((px for px in _all if px["tokenAddress"] == _a), None)
                            if _sell_pos:
                                _bp = sf(_sell_pos.get("buyPrice"))
                                _pnl = ((_cp - _bp) / _bp * 100) if _bp > 0 and _cp > 0 else 0
                                try:
                                    _sp_sol = get_sol_price()
                                except Exception:
                                    _sp_sol = 0
                                _sell(_sell_pos, 1, f"RISK:{flag[:30]}", _pnl, _sp_sol)
                                # Remove from positions
                                with _state_lock:
                                    _all2 = load_positions()
                                    save_positions([px for px in _all2 if px["tokenAddress"] != _a])
                            break
                except Exception:
                    pass
            threading.Thread(target=_run_rc, daemon=True).start()

    except Exception as e:
        log("ERROR", f"monitor: {e}")
    finally:
        _mon_busy = False


# ── Sell ────────────────────────────────────────────────────────────────

def _sell(pos, ratio, reason, pnl, sp):
    sym = pos["tokenSymbol"]
    addr = pos["tokenAddress"]
    hold = sf(pos.get("holdAmount"))
    sell_amt = hold * ratio
    dec = int(sf(pos.get("decimal"), 9))
    lam = round(sell_amt * (10 ** dec))
    b_sol = sf(pos.get("buyAmountSol"))
    p_sol = b_sol * (pnl / 100) * ratio
    tx_hash = ""

    if C.MODE == "paper":
        try:
            get_quote(addr, SOL_NATIVE, str(lam))
        except Exception:
            pass
    else:
        # Urgent exit (Ranking Exit / Hard SL / Engine Stop) uses higher slippage to ensure fill
        is_urgent = reason in ("RankExit", "StopExit", "QuickSL") or reason.startswith("SL(") or reason.startswith("RISK:")
        slippage = C.SLIPPAGE_SELL_URGENT if is_urgent else C.SLIPPAGE_SELL
        tx_hash = _live_sell(addr, lam, slippage, sym)

    pos["holdAmount"] = str(hold - sell_amt)
    add_trade(_make_trade("sell", addr, sym, f"{abs(p_sol + b_sol * ratio):.6f}", str(sell_amt),
                          pos.get("lastCheckPrice", "0"), tx_hash, reason, f"{pnl:.2f}", f"{p_sol:.6f}"))

    td = today_stats()
    td["pnlSol"] += p_sol
    td["trades"] += 1
    if p_sol >= 0:
        td["wins"] += 1
        _record_session_win()
    else:
        td["losses"] += 1
        _record_session_loss(abs(p_sol))
    update_today(td)

    if ratio >= 1 or pnl < 0:
        _cooldown[addr] = time.time() * 1000

    # Cleanup old cooldowns
    now = time.time() * 1000
    for k in list(_cooldown):
        if now - _cooldown[k] > 86400000:
            del _cooldown[k]


def _live_sell(addr, lamports, slippage, sym):
    w_addr = _wallet_cache or wallet_addr()
    try:
        return _exec_sell(addr, lamports, slippage, w_addr)
    except Exception as e1:
        log("WARN", f"{sym}: full sell failed ({e1}), trying batch 50%+50%...")
        half = lamports // 2
        rest = lamports - half
        tx_hash = ""
        try:
            tx_hash = _exec_sell(addr, half, slippage, w_addr)
            log("SELL", f"{sym}: batch 1/2 OK (tx: {tx_hash})")
        except Exception as e2:
            raise RuntimeError(f"batch sell 1/2 failed: {e2}")
        time.sleep(2)
        try:
            tx2 = _exec_sell(addr, rest, slippage, w_addr)
            log("SELL", f"{sym}: batch 2/2 OK (tx: {tx2})")
        except Exception as e3:
            log("WARN", f"{sym}: batch 2/2 failed ({e3}), partial sell only")
        return tx_hash


def _exec_sell(addr, lamports, slippage, w_addr):
    sw = get_swap(addr, SOL_NATIVE, str(lamports), w_addr, slippage)
    tx_data = (sw or {}).get("tx", {})
    if not tx_data.get("data"):
        raise RuntimeError(f"No sell callData: {json.dumps(sw)[:200]}")
    r = sign_and_send(tx_data["data"], tx_data["to"])
    if not r["success"]:
        raise RuntimeError(f"sell tx failed: {r['error']}")
    return r.get("txHash", "")


# ── Layer 3: Unconfirmed positions ──────────────────────────────────────

def _check_unconfirmed():
    if not _unconfirmed:
        return
    TIMEOUT_MS = 180000
    MAX_ZERO = 10

    for addr in list(_unconfirmed):
        entry = _unconfirmed[addr]
        pos = entry["pos"]
        elapsed = time.time() * 1000 - entry["start_time"]

        bal = query_token_balance(addr)

        if bal > 0:
            pos["holdAmount"] = str(bal)
            pos["unconfirmed"] = False
            pos["triggerReason"] = pos["triggerReason"].replace(" (unconfirmed)", " (confirmed)")
            with _state_lock:
                all_pos = load_positions()
                all_pos.append(pos)
                save_positions(all_pos)
            del _unconfirmed[addr]
            log("CONFIRMED", f"{pos['tokenSymbol']} | balance: {bal} | confirmed after {elapsed / 1000:.0f}s")
            continue

        if bal == -1:
            log("WARN", f"{pos['tokenSymbol']}: RPC error checking balance, skipping")
            continue

        entry["zero_count"] += 1
        if entry["zero_count"] >= MAX_ZERO and elapsed > TIMEOUT_MS:
            del _unconfirmed[addr]
            log("UNCONFIRMED_EXPIRED", f"{pos['tokenSymbol']} | {entry['zero_count']} zero checks over {elapsed / 1000:.0f}s -> position discarded")
        else:
            log("UNCONFIRMED", f"{pos['tokenSymbol']} | zero #{entry['zero_count']}/{MAX_ZERO} | {elapsed / 1000:.0f}s/{TIMEOUT_MS / 1000:.0f}s")


# ── Layer 4: Wallet Audit ───────────────────────────────────────────────

def _audit_loop():
    while not _stop_event.is_set():
        _stop_event.wait(C.HEALTH_CHECK_SEC)
        if _running and C.MODE == "live":
            _wallet_audit()


def _wallet_audit():
    global _audit_busy
    if _audit_busy:
        return
    _audit_busy = True
    try:
        if C.MODE != "live":
            return

        pos = load_positions()
        wallet_tokens = get_wallet_tokens()

        if len(wallet_tokens) == 0 and len(pos) > 0:
            log("AUDIT", f"Skipped - wallet API returned 0 tokens but we have {len(pos)} position(s)")
            return

        wallet_map = {}
        for wt in wallet_tokens:
            # [C3] tokenContractAddress is None in wallet balance — use tokenAddress
            wt_addr = wt.get("tokenAddress") or wt.get("tokenContractAddress") or ""
            if wt_addr:
                wallet_map[wt_addr] = sf(wt.get("balance"))

        drifts = 0
        AUDIT_COOLDOWN_MS = 300000

        for p in pos:
            addr = p["tokenAddress"]
            wallet_bal = wallet_map.get(addr)

            if wallet_bal is None or wallet_bal <= 0:
                age = time.time() * 1000 - (p.get("buyTimestamp", 0) or 0)
                if age < AUDIT_COOLDOWN_MS:
                    log("AUDIT", f"{p['tokenSymbol']}: not in wallet but only {age / 1000:.0f}s old, keeping (cooldown)")
                    continue
                direct_bal = query_token_balance(addr)
                if direct_bal > 0:
                    log("AUDIT", f"{p['tokenSymbol']}: not in wallet list but direct query shows {direct_bal}, keeping")
                    p["holdAmount"] = str(direct_bal)
                    drifts += 1
                    continue
                if direct_bal == -1:
                    log("AUDIT", f"{p['tokenSymbol']}: RPC error on direct check, keeping")
                    continue
                log("AUDIT", f"Ghost: {p['tokenSymbol']} in positions but NOT in wallet -> removing")
                p["_remove"] = True
                drifts += 1
            else:
                local_bal = sf(p.get("holdAmount"))
                diff = abs(wallet_bal - local_bal)
                if local_bal > 0 and diff / local_bal > 0.01:
                    log("AUDIT", f"Drift: {p['tokenSymbol']} local={local_bal} chain={wallet_bal} -> correcting")
                    p["holdAmount"] = str(wallet_bal)
                    drifts += 1

        # Check for orphaned tokens
        pos_addrs = set(p["tokenAddress"] for p in pos)
        all_trades = load_trades()
        buy_map = {}
        for t in all_trades:
            if t.get("direction") == "buy":
                buy_map[t.get("tokenAddress", "")] = t

        for wt in wallet_tokens:
            # [C3] use tokenAddress (tokenContractAddress is None)
            wt_addr = wt.get("tokenAddress") or wt.get("tokenContractAddress") or ""
            if wt_addr in pos_addrs or wt_addr in _unconfirmed:
                continue
            buy_trade = buy_map.get(wt_addr)
            if not buy_trade:
                continue
            hold_amt = sf(wt.get("balance"))
            if hold_amt <= 0:
                continue
            pos.append({
                "tokenAddress": wt_addr,
                "tokenSymbol": wt.get("symbol") or buy_trade.get("tokenSymbol", "?"),
                "decimal": int(sf(wt.get("decimal"), 9)),
                "buyPrice": buy_trade.get("priceUsd", "0"),
                "buyAmountSol": buy_trade.get("amountSol", "0"),
                "holdAmount": str(hold_amt),
                "buyCount": 1,
                "buyTimestamp": buy_trade.get("timestamp", int(time.time() * 1000)),
                "lastCheckPrice": buy_trade.get("priceUsd", "0"),
                "lastCheckTime": int(time.time() * 1000),
                "peakPrice": buy_trade.get("priceUsd", "0"),
                "takeProfitTier": 0,
                "triggerReason": "Recovered(audit)",
                "safetyData": {},
            })
            drifts += 1
            log("AUDIT", f"Recovered orphan: {wt.get('symbol') or wt_addr[:8]} ({hold_amt})")

        if drifts > 0:
            cleaned = [p for p in pos if not p.get("_remove")]
            for p in cleaned:
                p.pop("_remove", None)
            save_positions(cleaned)
            log("AUDIT", f"Fixed {drifts} drift(s), {len(cleaned)} active position(s)")

    except Exception as e:
        log("WARN", f"Wallet audit failed: {e}")
    finally:
        _audit_busy = False


# ── HTTP Server ─────────────────────────────────────────────────────────

class DashHandler(BaseHTTPRequestHandler):

    def log_message(self, fmt, *args):
        pass  # suppress access logs

    def _cors(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")

    def _json(self, data, status=200):
        body = json.dumps(data, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self._cors()
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self):
        self.send_response(204)
        self._cors()
        self.end_headers()

    def do_GET(self):
        path = self.path.split("?")[0]

        if path == "/" or path == "/index.html":
            html_path = PROJECT_DIR / "dashboard.html"
            if html_path.exists():
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                self.wfile.write(html_path.read_bytes())
            else:
                self.send_response(404)
                self.end_headers()
                self.wfile.write(b"dashboard.html not found")
            return

        if path == "/api/state":
            s = engine_state()
            if s["mode"] == "live":
                try:
                    s["wallet"] = wallet_addr()
                    s["solBalance"] = sol_balance()
                except Exception:
                    pass
            s["positions"] = load_positions()
            s["trades"] = load_trades()
            s["logs"] = get_logs(50)
            s["roster"] = get_roster()
            self._json(s)
            return

        if path == "/health":
            self._json({"ok": True})
            return

        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        path = self.path.split("?")[0]
        body = {}
        cl = int(self.headers.get("Content-Length", 0))
        if cl > 0:
            try:
                body = json.loads(self.rfile.read(cl))
            except Exception:
                pass

        if path == "/api/start":
            self._json(engine_start())
            return

        if path == "/api/stop":
            self._json(engine_stop())
            return

        if path == "/api/mode":
            mode = body.get("mode")
            if mode not in ("paper", "live"):
                self._json({"ok": False, "msg": 'mode must be "paper" or "live"'})
                return
            if _running:
                self._json({"ok": False, "msg": "Stop engine before switching mode"})
                return
            if mode == "live":
                try:
                    wallet_addr()
                except Exception as e:
                    self._json({"ok": False, "msg": f"Live mode requires onchainos wallet login: {e}"})
                    return
            C.MODE = mode
            self._json({"ok": True, "msg": f"Mode switched to {mode}"})
            return

        if path == "/api/reset":
            if _running:
                self._json({"ok": False, "msg": "Stop engine before reset"})
                return
            mode = C.MODE
            state_write(f"{mode}/positions.json", [])
            state_write(f"{mode}/trades.json", [])
            state_write(f"{mode}/daily-stats.json", {})
            state_write(f"{mode}/signals-log.json", [])
            label = "Paper" if mode == "paper" else "Live"
            self._json({"ok": True, "msg": f"{label} data cleared"})
            return

        self.send_response(404)
        self.end_headers()


# ── Main ────────────────────────────────────────────────────────────────

def main():
    print("=" * 60)
    print("  Top Rank Tokens Sniper v1.0")
    print(f"  Mode: {C.MODE}")
    print(f"  Budget: {C.TOTAL_BUDGET} SOL | Per Trade: {C.BUY_AMOUNT} SOL")
    print(f"  Max Positions: {C.MAX_POSITIONS}")
    print(f"  Dashboard: http://localhost:{C.DASHBOARD_PORT}")
    print("=" * 60)

    _check_onchainos()
    _ensure_dir(STATE_DIR / "paper")
    _ensure_dir(STATE_DIR / "live")

    server = HTTPServer(("127.0.0.1", C.DASHBOARD_PORT), DashHandler)
    print(f"\n  Dashboard ready: http://localhost:{C.DASHBOARD_PORT}\n")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n  Shutting down...")
        if _running:
            engine_stop()
        server.server_close()
        print("  Done.")


if __name__ == "__main__":
    main()
