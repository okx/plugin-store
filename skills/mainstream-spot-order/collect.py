#!/usr/bin/env python3
"""Candle data collector for base token + BTC overlay bars (15m).

Usage:
    python3 collect.py --pair SOL              # one-shot: append latest bars
    python3 collect.py --pair ETH --backfill   # paginate backwards, fetch max history
    python3 collect.py --pair SOL --daemon     # loop forever, fetch every 15m
"""
from __future__ import annotations

import argparse
import csv
import json
import os
import sys
import threading
import time
from http.server import HTTPServer, BaseHTTPRequestHandler

import config
import okx
import strategy

DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")
STATE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "state")

HEADER = ["ts", "o", "h", "l", "c", "vol"]


def _build_tokens() -> list[tuple[str, str, str, str | None]]:
    """Build [(mint, csv_path, label, chain_index_override), ...] for active pair.
    chain_index_override is None unless BTC overlay is fetched from a different chain.
    """
    p = config.pair()
    symbol = p["base_symbol"].lower()
    base_csv = os.path.join(DATA_DIR, f"{symbol}_15m.csv")
    tokens = [(p["base_mint"], base_csv, p["base_symbol"], None)]

    # BTC overlay
    if config.ACTIVE_PAIR == "BTC":
        # BTC pair: base IS BTC, no separate overlay needed
        pass
    elif p["chain_index"] == config.BTC_OVERLAY_CHAIN:
        # Same chain as BTC overlay (Ethereum) — use BTC overlay mint directly
        btc_csv = os.path.join(DATA_DIR, "btc_15m.csv")
        tokens.append((config.BTC_OVERLAY_MINT, btc_csv, "BTC", None))
    elif p["chain_family"] == "solana":
        # Solana — use the Solana WBTC reference
        btc_csv = os.path.join(DATA_DIR, "btc_15m.csv")
        tokens.append((config.WBTC_MINT, btc_csv, "BTC", None))
    else:
        # Other EVM chains — fetch BTC overlay from Ethereum
        btc_csv = os.path.join(DATA_DIR, "btc_15m.csv")
        tokens.append((config.BTC_OVERLAY_MINT, btc_csv, "BTC", config.BTC_OVERLAY_CHAIN))

    return tokens


# --------------- dashboard state ---------------
_state_lock = threading.Lock()
_state: dict = {
    "base_bars": 0, "btc_bars": 0,
    "base_price": None, "btc_price": None,
    "base_24h_pct": None, "btc_24h_pct": None,
    "base_chart": [], "btc_chart": [],
    "last_fetch_ts": None, "next_fetch_ts": None,
    "started_ts": None,
    "logs": [],
    "backtest": {},
    "pair_symbol": "",
    "pair_label": "",
}
DASH_HTML = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dashboard.html")


def _results_json_path() -> str:
    symbol = config.pair()["base_symbol"].lower()
    return os.path.join(os.path.dirname(os.path.abspath(__file__)), "results", f"latest_{symbol}.json")


def _log(msg: str):
    """Append timestamped message to _state logs (max 50)."""
    ts = time.strftime("%H:%M:%S")
    line = f"{ts} {msg}"
    with _state_lock:
        _state["logs"].append(line)
        if len(_state["logs"]) > 50:
            _state["logs"] = _state["logs"][-50:]
    print(line)


def _update_csv_stats():
    """Read CSV files and update _state with bar counts, prices, 24h change, sparklines."""
    tokens = _build_tokens()
    for idx, (mint, csv_path, label, _chain_override) in enumerate(tokens):
        is_base = (idx == 0)
        price_key = "base_price" if is_base else "btc_price"
        bars_key = "base_bars" if is_base else "btc_bars"
        pct_key = "base_24h_pct" if is_base else "btc_24h_pct"
        chart_key = "base_chart" if is_base else "btc_chart"

        rows = _load_existing(csv_path)
        if not rows:
            continue
        sorted_ts = sorted(rows.keys())
        count = len(sorted_ts)
        last_close = float(rows[sorted_ts[-1]][4])
        # 24h change: 96 bars ago (96 * 15m = 24h)
        idx_24h = max(0, len(sorted_ts) - 96)
        close_24h = float(rows[sorted_ts[idx_24h]][4])
        pct = (last_close - close_24h) / close_24h if close_24h else None
        # sparkline: last 96 closes
        chart = [float(rows[sorted_ts[i]][4]) for i in range(max(0, len(sorted_ts) - 96), len(sorted_ts))]
        with _state_lock:
            _state[bars_key] = count
            _state[price_key] = last_close
            _state[pct_key] = pct
            _state[chart_key] = chart


def _load_live_state() -> dict:
    """Load live_state_{pair}.json for position/trade data."""
    symbol = config.pair()["base_symbol"].lower()
    path = os.path.join(STATE_DIR, f"live_state_{symbol}.json")
    if not os.path.exists(path):
        return {}
    try:
        with open(path, "r") as f:
            return json.load(f)
    except (json.JSONDecodeError, OSError):
        return {}


def _compute_analytics():
    """Run strategy.analyze() on current bar data and update _state."""
    tokens = _build_tokens()
    if not tokens:
        return

    # Load base bars from CSV
    base_mint, base_csv, base_label, _ = tokens[0]
    base_rows = _load_existing(base_csv)
    if not base_rows:
        return

    sorted_ts = sorted(base_rows.keys())
    sol_bars = []
    for ts in sorted_ts:
        row = base_rows[ts]
        sol_bars.append({
            "ts": int(row[0]), "o": float(row[1]), "h": float(row[2]),
            "l": float(row[3]), "c": float(row[4]), "vol": float(row[5]),
        })

    # Load BTC bars
    btc_bars = []
    if len(tokens) > 1:
        btc_mint, btc_csv, btc_label, _ = tokens[1]
        btc_rows = _load_existing(btc_csv)
        btc_sorted = sorted(btc_rows.keys())
        for ts in btc_sorted:
            row = btc_rows[ts]
            btc_bars.append({
                "ts": int(row[0]), "o": float(row[1]), "h": float(row[2]),
                "l": float(row[3]), "c": float(row[4]), "vol": float(row[5]),
            })
    else:
        btc_bars = sol_bars  # BTC pair

    # Get strategy state from live state
    live = _load_live_state()
    strat_state = live.get("strategy_state", strategy.init_state())

    # Compute analytics
    analytics = strategy.analyze(strat_state, sol_bars, btc_bars)

    # Load position/trade info from live state
    position_info = {
        "position": live.get("position", 0.0),
        "daily_pnl": live.get("daily_pnl", 0.0),
        "paper_usdc": live.get("paper_usdc", config.INITIAL_USDC),
        "paper_base": live.get("paper_base", 0.0),
        "paper_entry_price": live.get("paper_entry_price", 0.0),
        "trades": live.get("trades", [])[-20:],  # last 20 trades
        "last_trade_ts": live.get("last_trade_ts", 0),
    }

    # Paper equity
    price = analytics.get("price", 0)
    if price > 0:
        position_info["paper_equity"] = position_info["paper_usdc"] + position_info["paper_base"] * price
    else:
        position_info["paper_equity"] = position_info["paper_usdc"]

    with _state_lock:
        _state["analytics"] = analytics
        _state["live"] = position_info
        _state["paper_mode"] = getattr(config, "PAPER_TRADE", False)


def _load_backtest():
    """Load results/latest_{symbol}.json into _state['backtest']."""
    path = _results_json_path()
    if not os.path.exists(path):
        return
    try:
        with open(path, "r") as f:
            data = json.load(f)
        bt = {k: data[k] for k in ("total_bars", "num_trades", "final_equity",
                                     "total_return", "max_drawdown", "sharpe",
                                     "buy_and_hold_return") if k in data}
        with _state_lock:
            _state["backtest"] = bt
    except Exception:
        pass


class DashHandler(BaseHTTPRequestHandler):
    """Serves dashboard.html and /api/state JSON."""

    def do_GET(self):
        if self.path == "/api/state":
            with _state_lock:
                body = json.dumps(_state).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/" or self.path == "/index.html":
            try:
                with open(DASH_HTML, "rb") as f:
                    body = f.read()
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            except FileNotFoundError:
                self.send_error(404, "dashboard.html not found")
        else:
            self.send_error(404)

    def log_message(self, format, *args):
        pass  # suppress access logs


def _start_dashboard():
    """Start the dashboard HTTP server in a daemon thread."""
    HTTPServer.allow_reuse_address = True
    server = HTTPServer(("0.0.0.0", config.DASHBOARD_PORT), DashHandler)
    t = threading.Thread(target=server.serve_forever, daemon=True)
    t.start()
    print(f"Dashboard running on http://localhost:{config.DASHBOARD_PORT}")


def _load_existing(path: str) -> dict[int, list]:
    """Load CSV into {ts: row} dict."""
    rows = {}
    if not os.path.exists(path):
        return rows
    with open(path, "r") as f:
        reader = csv.reader(f)
        hdr = next(reader, None)
        for row in reader:
            if len(row) >= 6:
                try:
                    ts = int(row[0])
                    rows[ts] = row
                except ValueError:
                    continue
    return rows


def _write_csv(path: str, rows: dict[int, list]):
    """Write rows dict to CSV, sorted by timestamp."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    sorted_ts = sorted(rows.keys())
    with open(path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(HEADER)
        for ts in sorted_ts:
            writer.writerow(rows[ts])


def _parse_candle(raw) -> tuple[int, list] | None:
    """Parse a candle from CLI or REST format into (ts, [ts,o,h,l,c,vol]).
    CLI returns dicts, REST returns lists.
    """
    if isinstance(raw, dict):
        try:
            ts = int(raw.get("ts", raw.get("timestamp", 0)))
            o = raw.get("o", raw.get("open", "0"))
            h = raw.get("h", raw.get("high", "0"))
            l = raw.get("l", raw.get("low", "0"))
            c = raw.get("c", raw.get("close", "0"))
            vol = raw.get("vol", raw.get("volume", raw.get("baseVolume", "0")))
            return ts, [str(ts), str(o), str(h), str(l), str(c), str(vol)]
        except (ValueError, TypeError):
            return None
    elif isinstance(raw, list) and len(raw) >= 6:
        try:
            ts = int(raw[0])
            return ts, [str(raw[0]), str(raw[1]), str(raw[2]),
                        str(raw[3]), str(raw[4]), str(raw[5])]
        except (ValueError, TypeError, IndexError):
            return None
    return None


def fetch_latest(token: str, csv_path: str, label: str,
                 chain_index: str | None = None):
    """Fetch latest bars via CLI and append new ones."""
    existing = _load_existing(csv_path)
    candles = okx.kline(token, config.BAR_SIZE, limit=299, chain_index=chain_index)
    added = 0
    for raw in candles:
        parsed = _parse_candle(raw)
        if parsed:
            ts, row = parsed
            if ts not in existing:
                existing[ts] = row
                added += 1
    _write_csv(csv_path, existing)
    print(f"[{label}] {added} new bars appended, {len(existing)} total")


def backfill(token: str, csv_path: str, label: str,
             chain_index: str | None = None):
    """Paginate backwards via REST API to fetch max history."""
    existing = _load_existing(csv_path)
    print(f"[{label}] Starting backfill, {len(existing)} existing bars...")

    # First fetch latest via CLI
    try:
        candles = okx.kline(token, config.BAR_SIZE, limit=299, chain_index=chain_index)
        for raw in candles:
            parsed = _parse_candle(raw)
            if parsed:
                ts, row = parsed
                existing[ts] = row
    except Exception as e:
        print(f"[{label}] CLI kline failed: {e}, continuing with REST only")

    # Paginate backwards via REST
    after = 0  # 0 = start from latest
    empty_pages = 0
    max_pages = 100  # safety limit

    for page in range(max_pages):
        try:
            candles = okx.kline_history(token, config.BAR_SIZE, limit=299, after=after,
                                        chain_index=chain_index)
        except Exception as e:
            print(f"[{label}] REST page {page} error: {e}")
            time.sleep(2)
            empty_pages += 1
            if empty_pages >= 3:
                break
            continue

        if not candles:
            empty_pages += 1
            if empty_pages >= 3:
                break
            time.sleep(1)
            continue

        empty_pages = 0
        oldest_ts = None
        added_this_page = 0
        for raw in candles:
            parsed = _parse_candle(raw)
            if parsed:
                ts, row = parsed
                if ts not in existing:
                    added_this_page += 1
                existing[ts] = row
                if oldest_ts is None or ts < oldest_ts:
                    oldest_ts = ts

        if oldest_ts:
            after = oldest_ts
        else:
            break

        if page % 10 == 0:
            print(f"[{label}] Page {page}: {len(existing)} bars, oldest={after}")

        time.sleep(0.3)  # rate limit

    _write_csv(csv_path, existing)
    print(f"[{label}] Backfill done: {len(existing)} total bars")


def daemon():
    """Loop forever, fetching every 15 minutes."""
    p = config.pair()
    print(f"Daemon mode [{p['label']}]: fetching every 15 minutes. Ctrl+C to stop.")

    tokens = _build_tokens()

    # Start dashboard
    with _state_lock:
        _state["started_ts"] = time.time()
        _state["pair_symbol"] = p["base_symbol"]
        _state["pair_label"] = p["label"]
    _load_backtest()
    _update_csv_stats()
    _compute_analytics()
    _start_dashboard()

    while True:
        for mint, csv_path, label, chain_override in tokens:
            try:
                fetch_latest(mint, csv_path, label, chain_index=chain_override)
                rows = _load_existing(csv_path)
                _log(f"[{label}] fetched, {len(rows)} total bars")
            except Exception as e:
                _log(f"[{label}] Error: {e}")

        # Update dashboard state
        _update_csv_stats()
        _compute_analytics()
        with _state_lock:
            _state["last_fetch_ts"] = time.time()

        # Sleep until next bar boundary + 30s settling
        now = time.time()
        next_bar = (int(now) // config.BAR_SECONDS + 1) * config.BAR_SECONDS
        sleep_secs = (next_bar - now) + 30

        with _state_lock:
            _state["next_fetch_ts"] = now + sleep_secs

        _log(f"Sleeping {sleep_secs:.0f}s until next bar...")
        time.sleep(sleep_secs)


def main():
    parser = argparse.ArgumentParser(description="Candle data collector")
    parser.add_argument("--pair", default="SOL",
                        help="Trading pair (default: SOL). See: python3 config.py --list")
    parser.add_argument("--backfill", action="store_true", help="Backfill historical data")
    parser.add_argument("--daemon", action="store_true", help="Run in daemon mode")
    args = parser.parse_args()

    config.ACTIVE_PAIR = args.pair.upper()
    tokens = _build_tokens()

    if args.backfill:
        for mint, csv_path, label, chain_override in tokens:
            backfill(mint, csv_path, label, chain_index=chain_override)
    elif args.daemon:
        daemon()
    else:
        for mint, csv_path, label, chain_override in tokens:
            fetch_latest(mint, csv_path, label, chain_index=chain_override)


if __name__ == "__main__":
    main()
