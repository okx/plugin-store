#!/usr/bin/env python3
"""Market Structure Analyzer v3.0 — Live Dashboard Server.

Usage:
    python3 msa_server.py              # start on default port (8420)
    python3 msa_server.py --port 9000  # custom port

Serves a live SPA dashboard with K-line charts, TA indicators, and
composite signal scoring. Background threads poll via OKX CeFi CLI
(`okx market`) for fresh data. Original CLI usage is unaffected.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import time
from datetime import datetime, timezone
from http.server import HTTPServer, BaseHTTPRequestHandler
from socketserver import ThreadingMixIn
from concurrent.futures import ThreadPoolExecutor, as_completed
from urllib.parse import urlparse, parse_qs

# ── Resolve imports ────────────────────────────────────────────────
SKILL_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, SKILL_DIR)
sys.path.insert(0, os.path.join(SKILL_DIR, "scripts"))

import config as cfg
import fetch_market_data as fmd

# ═══════════════════════════════════════════════════════════════════
# GLOBAL CACHES
# ═══════════════════════════════════════════════════════════════════

_lock = threading.Lock()
_structure_cache: dict = {}       # token → {data, ts}
_candle_cache: dict = {}          # (token, bar) → {payload, ts}
_macro_cache: dict = {}           # {data, ts}
_composite_scores: dict = {}      # token → {score, label, signals}
_ticker_cache: dict = {}          # token → {price, change_pct, high, low, vol, ts}

_hot_token = "BTC"
_hot_bar = "1H"
_running = True

TICKER_POLL_SEC = 3               # fast price ticker (direct HTTP, ~75ms)

# ═══════════════════════════════════════════════════════════════════
# COMPOSITE SIGNAL SCORING
# ═══════════════════════════════════════════════════════════════════

def _compute_composite(token: str) -> dict:
    """Compute composite signal score (-100 to +100) from cached data."""
    with _lock:
        sc = _structure_cache.get(token, {}).get("data", {})
        mc = _macro_cache.get("data", {})
        cc_key = (token, _hot_bar)
        candle_payload = _candle_cache.get(cc_key, {}).get("payload", {})

    if not sc:
        return {"score": 0, "label": "NO DATA", "signals": []}

    deriv = sc.get("derivatives", {})
    mstruct = sc.get("market_structure", {})
    macro = mc

    signals = []
    weights = []

    def add(name, weight, value):
        if value is not None:
            signals.append({"name": name, "weight": weight, "value": round(value, 2)})
            weights.append((weight, value))

    # 1. Funding Rate (15%)
    funding = deriv.get("funding", {})
    if funding.get("status") == "available" and funding.get("rate") is not None:
        rate = funding["rate"]
        if rate < -0.00005:
            add("Funding Rate", 15, 100)
        elif rate > 0.0002:
            add("Funding Rate", 15, -100)
        else:
            add("Funding Rate", 15, max(-100, min(100, -rate * 500000)))

    # 2. OI Delta 24h (10%)
    oih = deriv.get("oi_history", {})
    if oih.get("status") == "available" and oih.get("oi_delta_1d_pct") is not None:
        d = oih["oi_delta_1d_pct"]
        if d > 5:
            add("OI Delta 24h", 10, 100)
        elif d < -5:
            add("OI Delta 24h", 10, -100)
        else:
            add("OI Delta 24h", 10, d * 20)

    # 3. Futures Basis (10%)
    basis = deriv.get("basis", {})
    if basis.get("status") == "available" and basis.get("basis_pct") is not None:
        b = basis["basis_pct"]
        if 0 < b <= 0.05:
            add("Futures Basis", 10, 60)
        elif b < 0:
            add("Futures Basis", 10, -80)
        else:
            add("Futures Basis", 10, max(-100, min(100, -b * 200 + 100)))

    # 4. Taker Buy/Sell (15%)
    tv = mstruct.get("taker_volume", {})
    if tv.get("status") == "available" and tv.get("buy_sell_ratio") is not None:
        r = tv["buy_sell_ratio"]
        if r > 1.05:
            add("Taker Buy/Sell", 15, min(100, (r - 1) * 1000))
        elif r < 0.95:
            add("Taker Buy/Sell", 15, max(-100, (r - 1) * 1000))
        else:
            add("Taker Buy/Sell", 15, (r - 1) * 1000)

    # 5. RSI 1H (10%)
    ta = candle_payload.get("ta", {})
    rsi_list = ta.get("rsi", [])
    if rsi_list:
        rsi_val = rsi_list[-1].get("value", 50)
        if 30 <= rsi_val <= 50:
            add("RSI", 10, 60)
        elif rsi_val > 70:
            add("RSI", 10, -80)
        elif rsi_val < 30:
            add("RSI", 10, 80)
        else:
            add("RSI", 10, max(-100, min(100, (50 - rsi_val) * 5)))

    # 6. MACD (10%)
    macd_data = ta.get("macd", {})
    hist_list = macd_data.get("histogram", [])
    if hist_list:
        h = hist_list[-1].get("value", 0)
        if h > 0:
            add("MACD", 10, min(100, h * 500))
        else:
            add("MACD", 10, max(-100, h * 500))

    # 7. Fear & Greed (10%)
    fng = macro.get("fear_greed", {})
    if fng.get("status") == "available" and fng.get("value") is not None:
        v = fng["value"]
        if v < 25:
            add("Fear & Greed", 10, 80)   # extreme fear = contrarian bullish
        elif v > 75:
            add("Fear & Greed", 10, -80)  # greed = contrarian bearish
        else:
            add("Fear & Greed", 10, (50 - v) * 2)

    # 8. Long/Short (5%)
    ls = mstruct.get("long_short", {})
    if ls.get("status") == "available":
        long_r = ls.get("long_ratio")
        if long_r is not None:
            if long_r < 0.48:
                add("Long/Short", 5, 70)
            elif long_r > 0.55:
                add("Long/Short", 5, -70)
            else:
                add("Long/Short", 5, (0.5 - long_r) * 500)

    # 9. Funding Trend (5%)
    fh = deriv.get("funding_history", {})
    if fh.get("status") == "available" and fh.get("trend"):
        t = fh["trend"]
        if t == "decreasing":
            add("Funding Trend", 5, 60)
        elif t == "increasing":
            add("Funding Trend", 5, -60)
        else:
            add("Funding Trend", 5, 0)

    # 10. Options Skew (5%) — T1 tokens only
    skew = deriv.get("skew", {})
    if skew.get("status") == "available" and skew.get("skew_25d") is not None:
        s = skew["skew_25d"]
        if s < 0:
            add("Options Skew", 5, 60)
        elif s > 5:
            add("Options Skew", 5, -60)
        else:
            add("Options Skew", 5, -s * 12)

    # 11. MVRV (5%) — T1 tokens only
    onchain = sc.get("on_chain", {})
    mvrv = onchain.get("mvrv", {})
    if mvrv.get("status") == "available" and mvrv.get("mvrv") is not None:
        m = mvrv["mvrv"]
        if m < 1.5:
            add("MVRV", 5, 80)
        elif m > 3.0:
            add("MVRV", 5, -80)
        else:
            add("MVRV", 5, max(-100, min(100, (2.25 - m) * 100)))

    # 12. Smart Money Flow (5%) — OnchainOS
    sm = macro.get("smart_money", {})
    if sm.get("status") == "available" and sm.get("buy_pct") is not None:
        bp = sm["buy_pct"]
        # >65% buying = bullish, <35% = bearish
        if bp > 65:
            add("Smart Money", 5, 80)
        elif bp < 35:
            add("Smart Money", 5, -80)
        else:
            add("Smart Money", 5, (bp - 50) * 5)

    # Renormalize weights
    if not weights:
        return {"score": 0, "label": "NO DATA", "signals": signals}

    total_weight = sum(w for w, _ in weights)
    score = sum(w * v for w, v in weights) / total_weight if total_weight > 0 else 0
    score = max(-100, min(100, score))

    if score > 25:
        label = "BULLISH"
    elif score > 5:
        label = "LEAN BULLISH"
    elif score < -25:
        label = "BEARISH"
    elif score < -5:
        label = "LEAN BEARISH"
    else:
        label = "NEUTRAL"

    return {"score": round(score, 1), "label": label, "signals": signals}


# ═══════════════════════════════════════════════════════════════════
# BACKGROUND THREADS
# ═══════════════════════════════════════════════════════════════════

def _structure_loop():
    """Poll structure indicators for hot token every STRUCTURE_POLL_SEC."""
    global _running
    while _running:
        try:
            token = _hot_token
            tm = fmd.TOKEN_MAP.get(token)
            if tm:
                data = fmd.analyze_token(token)
                with _lock:
                    _structure_cache[token] = {"data": data, "ts": time.time()}
                # Update composite
                score = _compute_composite(token)
                with _lock:
                    _composite_scores[token] = score
                _log(f"[structure] {token} refreshed — score {score['score']} {score['label']}")
        except Exception as e:
            _log(f"[structure] error: {e}")
        _sleep(cfg.STRUCTURE_POLL_SEC)


def _macro_loop():
    """Poll macro indicators every STRUCTURE_POLL_SEC — concurrent fetches."""
    global _running
    while _running:
        try:
            tasks = {
                "fear_greed": fmd.fear_greed,
                "global": fmd.coingecko_global,
                "stablecoins": fmd.stablecoin_data,
                "smart_money": fmd.onchain_smart_money_signals,
                "hot_tokens_dex": fmd.onchain_hot_tokens,
            }
            macro = {}
            with ThreadPoolExecutor(max_workers=5) as pool:
                futures = {pool.submit(fn): name for name, fn in tasks.items()}
                for fut in as_completed(futures):
                    name = futures[fut]
                    try:
                        macro[name] = fut.result(timeout=15)
                    except Exception:
                        macro[name] = {"status": "unavailable"}
            with _lock:
                _macro_cache["data"] = macro
                _macro_cache["ts"] = time.time()
            _log("[macro] refreshed (concurrent)")
        except Exception as e:
            _log(f"[macro] error: {e}")
        _sleep(cfg.STRUCTURE_POLL_SEC)


def _candle_loop():
    """Poll candles for hot token+bar every CANDLE_POLL_SEC."""
    global _running
    while _running:
        try:
            token = _hot_token
            bar = _hot_bar
            tm = fmd.TOKEN_MAP.get(token)
            if tm:
                payload = _fetch_candles(token, bar)
                if payload:
                    with _lock:
                        _candle_cache[(token, bar)] = {"payload": payload, "ts": time.time()}
                    # Re-compute composite (RSI/MACD may have changed)
                    score = _compute_composite(token)
                    with _lock:
                        _composite_scores[token] = score
                    _log(f"[candles] {token}/{bar} refreshed — {len(payload.get('candles', []))} bars")
        except Exception as e:
            _log(f"[candles] error: {e}")
        _sleep(cfg.CANDLE_POLL_SEC)


def _ticker_loop():
    """Fast price ticker — direct HTTP, polls every TICKER_POLL_SEC (~75ms per call)."""
    global _running
    while _running:
        try:
            token = _hot_token
            tm = fmd.TOKEN_MAP.get(token)
            if tm:
                data = fmd.okx_ticker_fast(tm["okx_spot"])
                if data and data.get("status") == "available":
                    with _lock:
                        _ticker_cache[token] = {
                            "price": data["price"],
                            "change_pct": data.get("price_change_pct", 0),
                            "high_24h": data.get("high_24h", 0),
                            "low_24h": data.get("low_24h", 0),
                            "volume_24h": data.get("volume_24h_quote", 0),
                            "ts": time.time(),
                        }
        except Exception:
            pass  # silent — fast loop, don't spam logs
        time.sleep(TICKER_POLL_SEC)


def _fetch_candles(token: str, bar: str) -> dict | None:
    """Fetch candles + compute TA indicators."""
    tm = fmd.TOKEN_MAP.get(token)
    if not tm:
        return None
    inst_id = tm["okx_spot"]
    candles = fmd.okx_candle_ohlcv(inst_id, bar=bar, limit=cfg.CANDLE_LIMIT)
    if not candles:
        return None
    ta = fmd.compute_ta_indicators(
        candles,
        rsi_period=cfg.RSI_PERIOD,
        bb_period=cfg.BB_PERIOD,
        bb_std=cfg.BB_STD,
        macd_fast=cfg.MACD_FAST,
        macd_slow=cfg.MACD_SLOW,
        macd_signal=cfg.MACD_SIGNAL,
    )
    return {"candles": candles, "ta": ta, "token": token, "bar": bar}


def _sleep(seconds: int):
    """Interruptible sleep."""
    deadline = time.time() + seconds
    while _running and time.time() < deadline:
        time.sleep(1)


def _log(msg: str):
    ts = datetime.now().strftime("%H:%M:%S")
    print(f"  {ts}  {msg}", file=sys.stderr)


# ═══════════════════════════════════════════════════════════════════
# HTTP SERVER
# ═══════════════════════════════════════════════════════════════════

class ThreadedHTTPServer(ThreadingMixIn, HTTPServer):
    daemon_threads = True


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass  # suppress default access log

    def _json(self, data: dict, status: int = 200):
        body = json.dumps(data).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)

    def _html(self, path: str):
        try:
            with open(path, "r", encoding="utf-8") as f:
                body = f.read().encode()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        except FileNotFoundError:
            self.send_error(404, "Not found")

    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path
        qs = parse_qs(parsed.query)

        if path == "/" or path == "/dashboard.html":
            self._html(os.path.join(SKILL_DIR, "dashboard.html"))

        elif path == "/api/state":
            self._handle_state(qs)

        elif path == "/api/candles":
            self._handle_candles(qs)

        elif path == "/api/ticker":
            self._handle_ticker(qs)

        elif path == "/api/set":
            self._handle_set(qs)

        else:
            self.send_error(404, "Not found")

    def _handle_state(self, qs):
        token = qs.get("token", [_hot_token])[0].upper()
        with _lock:
            sc = _structure_cache.get(token, {}).get("data", {})
            mc = _macro_cache.get("data", {})
            comp = _composite_scores.get(token, {})
            sc_ts = _structure_cache.get(token, {}).get("ts", 0)
            mc_ts = _macro_cache.get("ts", 0)

        self._json({
            "token": token,
            "structure": sc,
            "macro": mc,
            "composite": comp,
            "cache_age_structure": round(time.time() - sc_ts, 1) if sc_ts else None,
            "cache_age_macro": round(time.time() - mc_ts, 1) if mc_ts else None,
            "supported_tokens": sorted(fmd.TOKEN_MAP.keys()),
            "supported_bars": cfg.SUPPORTED_BARS,
            "hot_token": _hot_token,
            "hot_bar": _hot_bar,
        })

    def _handle_candles(self, qs):
        global _hot_token, _hot_bar
        token = qs.get("token", [_hot_token])[0].upper()
        bar = qs.get("bar", [_hot_bar])[0]
        if bar not in cfg.SUPPORTED_BARS:
            bar = "1H"

        cache_key = (token, bar)
        with _lock:
            cached = _candle_cache.get(cache_key)

        # Cache hit within TTL
        if cached and (time.time() - cached["ts"]) < cfg.CANDLE_POLL_SEC:
            self._json(cached["payload"])
            return

        # Cache miss — fetch inline
        payload = _fetch_candles(token, bar)
        if payload:
            with _lock:
                _candle_cache[cache_key] = {"payload": payload, "ts": time.time()}
            self._json(payload)
        else:
            self._json({"error": f"No candle data for {token}/{bar}"}, 404)

    def _handle_set(self, qs):
        """Update hot token/bar so background loops follow."""
        global _hot_token, _hot_bar
        token = qs.get("token", [None])[0]
        bar = qs.get("bar", [None])[0]
        changed = False
        if token and token.upper() in fmd.TOKEN_MAP:
            _hot_token = token.upper()
            changed = True
        if bar and bar in cfg.SUPPORTED_BARS:
            _hot_bar = bar
            changed = True
        self._json({"hot_token": _hot_token, "hot_bar": _hot_bar, "changed": changed})

    def _handle_ticker(self, qs):
        """Fast price ticker — returns cached price (updated every 5s)."""
        token = qs.get("token", [_hot_token])[0].upper()
        with _lock:
            cached = _ticker_cache.get(token)
        if cached:
            self._json({
                "token": token,
                "price": cached["price"],
                "change_pct": cached["change_pct"],
                "high_24h": cached["high_24h"],
                "low_24h": cached["low_24h"],
                "volume_24h": cached["volume_24h"],
                "age_ms": round((time.time() - cached["ts"]) * 1000),
            })
        else:
            self._json({"token": token, "price": None, "error": "no ticker data yet"})


# ═══════════════════════════════════════════════════════════════════
# STARTUP
# ═══════════════════════════════════════════════════════════════════

def main():
    global _running
    port = cfg.DASHBOARD_PORT

    # Parse --port arg
    args = sys.argv[1:]
    for i, a in enumerate(args):
        if a == "--port" and i + 1 < len(args):
            port = int(args[i + 1])

    print(f"\n  Market Structure Analyzer v3.0", file=sys.stderr)
    print(f"  Dashboard: http://localhost:{port}", file=sys.stderr)
    print(f"  Hot token: {_hot_token}  Bar: {_hot_bar}", file=sys.stderr)
    print(f"  Structure poll: {cfg.STRUCTURE_POLL_SEC}s  Candle poll: {cfg.CANDLE_POLL_SEC}s  Ticker poll: {TICKER_POLL_SEC}s\n", file=sys.stderr)

    # Initial load
    _log("Loading initial data...")
    try:
        data = fmd.analyze_token(_hot_token)
        with _lock:
            _structure_cache[_hot_token] = {"data": data, "ts": time.time()}
        _log(f"[init] {_hot_token} structure loaded")
    except Exception as e:
        _log(f"[init] structure error: {e}")

    try:
        macro_tasks = {
            "fear_greed": fmd.fear_greed,
            "global": fmd.coingecko_global,
            "stablecoins": fmd.stablecoin_data,
            "smart_money": fmd.onchain_smart_money_signals,
            "hot_tokens_dex": fmd.onchain_hot_tokens,
        }
        macro = {}
        with ThreadPoolExecutor(max_workers=5) as pool:
            futures = {pool.submit(fn): name for name, fn in macro_tasks.items()}
            for fut in as_completed(futures):
                name = futures[fut]
                try:
                    macro[name] = fut.result(timeout=15)
                except Exception:
                    macro[name] = {"status": "unavailable"}
        with _lock:
            _macro_cache["data"] = macro
            _macro_cache["ts"] = time.time()
        _log("[init] macro loaded (concurrent)")
    except Exception as e:
        _log(f"[init] macro error: {e}")

    try:
        payload = _fetch_candles(_hot_token, _hot_bar)
        if payload:
            with _lock:
                _candle_cache[(_hot_token, _hot_bar)] = {"payload": payload, "ts": time.time()}
            _log(f"[init] candles loaded — {len(payload.get('candles', []))} bars")
    except Exception as e:
        _log(f"[init] candles error: {e}")

    # Initial ticker (direct HTTP — fast)
    try:
        tm = fmd.TOKEN_MAP.get(_hot_token)
        if tm:
            td = fmd.okx_ticker_fast(tm["okx_spot"])
            if td and td.get("status") == "available":
                with _lock:
                    _ticker_cache[_hot_token] = {
                        "price": td["price"],
                        "change_pct": td.get("price_change_pct", 0),
                        "high_24h": td.get("high_24h", 0),
                        "low_24h": td.get("low_24h", 0),
                        "volume_24h": td.get("volume_24h_quote", 0),
                        "ts": time.time(),
                    }
                _log(f"[init] ticker: ${td['price']:,.2f}")
    except Exception:
        pass

    # Compute initial composite
    score = _compute_composite(_hot_token)
    with _lock:
        _composite_scores[_hot_token] = score
    _log(f"[init] composite: {score['score']} {score['label']}")

    # Start background threads
    threads = [
        threading.Thread(target=_structure_loop, daemon=True, name="structure"),
        threading.Thread(target=_macro_loop, daemon=True, name="macro"),
        threading.Thread(target=_candle_loop, daemon=True, name="candles"),
        threading.Thread(target=_ticker_loop, daemon=True, name="ticker"),
    ]
    for t in threads:
        t.start()

    # Start HTTP server
    server = ThreadedHTTPServer(("0.0.0.0", port), Handler)
    _log(f"Serving on http://localhost:{port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        _running = False
        _log("Shutting down...")
        server.shutdown()


if __name__ == "__main__":
    main()
