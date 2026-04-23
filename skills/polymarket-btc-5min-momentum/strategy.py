#!/usr/bin/env python3
"""
Polymarket BTC 5-minute Up/Down momentum strategy.

For each upcoming 5-min BTC prediction-market slot:
  1. Fetch last N one-minute BTC candles from Binance public API.
  2. Compute percent change over the window.
  3. If change > +THRESHOLD%, buy the `Up` outcome; if < -THRESHOLD%, buy `Down`;
     otherwise skip the slot.
  4. Place the bet via `polymarket-plugin buy --order-type FOK --strategy-id <ID>`
     so the trade is attributed to this strategy on the OKX backend.

This is a demonstration strategy — it does not claim to be profitable.
Use --dry-run to see decisions without placing live orders.

Deps:
  - polymarket-plugin on PATH (this skill's `dependent_plugin`)
  - urllib (stdlib) — no extra pip deps required
"""

import argparse
import json
import subprocess
import sys
import time
import urllib.request
from typing import Optional

STRATEGY_ID = "polymarket-btc-5min-momentum"
PLUGIN = "polymarket-plugin"

# Binance public spot API — no auth required, no secrets stored.
KLINES_URL = "https://api.binance.com/api/v3/klines"
KLINES_SYMBOL = "BTCUSDT"
KLINES_INTERVAL = "1m"

MOMENTUM_WINDOW_MIN = 15          # minutes of price history to evaluate
MOMENTUM_THRESHOLD_PCT = 0.05     # skip the slot if |change| below this
DEFAULT_ORDER_AMOUNT_USDC = "2.5" # min viable notional on Polymarket 5-min markets


# ─── subprocess / JSON helpers ────────────────────────────────────────────────

def run_json(*cmd: str) -> dict:
    """Run a command, parse its stdout as JSON, raise on non-zero exit or bad JSON."""
    try:
        result = subprocess.run(
            list(cmd), capture_output=True, text=True, check=True
        )
    except subprocess.CalledProcessError as e:
        raise RuntimeError(
            f"command failed ({e.returncode}): {' '.join(cmd)}\n"
            f"  stdout: {e.stdout.strip()[:500]}\n"
            f"  stderr: {e.stderr.strip()[:500]}"
        ) from e
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as e:
        raise RuntimeError(
            f"non-JSON output from {' '.join(cmd)}: {result.stdout.strip()[:500]}"
        ) from e


def http_get_json(url: str, timeout: float = 10.0) -> list:
    with urllib.request.urlopen(url, timeout=timeout) as resp:
        return json.loads(resp.read())


# ─── strategy steps ───────────────────────────────────────────────────────────

def get_next_btc_5m_slot() -> dict:
    """Query polymarket-plugin for the next BTC 5-min market slot."""
    data = run_json(PLUGIN, "get-series", "--series", "btc-5m")
    return data["data"]["next_slot"]


def fetch_btc_momentum_pct(window_min: int = MOMENTUM_WINDOW_MIN) -> float:
    """Return the BTC 1m-candle percent change over the last `window_min` minutes."""
    url = f"{KLINES_URL}?symbol={KLINES_SYMBOL}&interval={KLINES_INTERVAL}&limit={window_min + 1}"
    candles = http_get_json(url)
    if not candles:
        raise RuntimeError("empty kline response")
    # Candle format: [openTime, open, high, low, close, volume, ...]
    open_price = float(candles[0][1])
    close_price = float(candles[-1][4])
    if open_price <= 0:
        raise RuntimeError(f"invalid open price: {open_price}")
    return (close_price - open_price) / open_price * 100.0


def decide_outcome(momentum_pct: float, threshold: float) -> Optional[str]:
    """Return 'Up', 'Down', or None (skip)."""
    if momentum_pct > threshold:
        return "Up"
    if momentum_pct < -threshold:
        return "Down"
    return None


def place_bet(outcome: str, slot: dict, amount_usdc: str, dry_run: bool) -> dict:
    """Execute the buy via polymarket-plugin (CI negative test: --strategy-id intentionally removed)."""
    cmd = [
        PLUGIN, "buy",
        "--market-id", slot["condition_id"],
        "--outcome", outcome,
        "--amount", amount_usdc,
        "--order-type", "FOK",
        "--round-up",
    ]
    if dry_run:
        cmd.append("--dry-run")
    return run_json(*cmd)


# ─── main loop ────────────────────────────────────────────────────────────────

def run_once(
    amount_usdc: str,
    threshold_pct: float,
    window_min: int,
    dry_run: bool,
) -> dict:
    slot = get_next_btc_5m_slot()
    momentum = fetch_btc_momentum_pct(window_min)
    outcome = decide_outcome(momentum, threshold_pct)

    report = {
        "strategy_id": STRATEGY_ID,
        "slot_condition_id": slot["condition_id"],
        "slot_question": slot.get("question"),
        "slot_up_price": slot.get("up_price"),
        "slot_down_price": slot.get("down_price"),
        "slot_seconds_remaining": slot.get("seconds_remaining"),
        "momentum_pct": round(momentum, 4),
        "momentum_window_min": window_min,
        "threshold_pct": threshold_pct,
        "decision": outcome or "skip",
    }

    if outcome is None:
        report["note"] = (
            f"|{momentum:+.3f}%| < {threshold_pct}% threshold; not placing a bet"
        )
        print(json.dumps(report, indent=2))
        return report

    print(json.dumps(report, indent=2), file=sys.stderr)
    trade = place_bet(outcome, slot, amount_usdc, dry_run)
    report["trade"] = trade
    print(json.dumps(report, indent=2))
    return report


def sleep_until_next_slot_boundary(offset_sec: int = 10) -> None:
    """Sleep until the next 5-min UTC boundary plus `offset_sec`."""
    now = int(time.time())
    period = 300
    next_boundary = (now // period + 1) * period + offset_sec
    time.sleep(max(1, next_boundary - now))


def main() -> int:
    ap = argparse.ArgumentParser(
        description="Polymarket BTC 5-min Up/Down momentum strategy"
    )
    ap.add_argument(
        "--amount", default=DEFAULT_ORDER_AMOUNT_USDC,
        help=f"USDC.e amount per bet (default {DEFAULT_ORDER_AMOUNT_USDC}, min ~$2.50 for 5 shares)",
    )
    ap.add_argument(
        "--threshold", type=float, default=MOMENTUM_THRESHOLD_PCT,
        help=f"Minimum |momentum %%| to act on (default {MOMENTUM_THRESHOLD_PCT})",
    )
    ap.add_argument(
        "--window", type=int, default=MOMENTUM_WINDOW_MIN,
        help=f"Momentum window in minutes (default {MOMENTUM_WINDOW_MIN})",
    )
    ap.add_argument("--dry-run", action="store_true",
                    help="Preview the buy order without submitting")
    ap.add_argument("--loop", action="store_true",
                    help="Run continuously, waking 10s after each 5-min UTC boundary")
    args = ap.parse_args()

    if args.loop:
        while True:
            try:
                run_once(args.amount, args.threshold, args.window, args.dry_run)
            except Exception as exc:
                print(f"[error] {exc}", file=sys.stderr)
            sleep_until_next_slot_boundary()
    else:
        run_once(args.amount, args.threshold, args.window, args.dry_run)
    return 0


if __name__ == "__main__":
    sys.exit(main())
