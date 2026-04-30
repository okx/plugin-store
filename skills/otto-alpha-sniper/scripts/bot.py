#!/usr/bin/env python3
"""
Otto Alpha Sniper v0.1 — Optional autonomous poller.

The reactive flow documented in SKILL.md (AI agent fires trades on user intent)
is the primary mode. This bot.py is for advanced users who want a hands-free
background poller that checks the Otto signal feed every N seconds and fires
trades when thresholds are met.

Run:
    python3 bot.py --mode trending --interval 300

    # Dry-run by default. Pass --live to submit real orders.
    python3 bot.py --mode funding-fade --interval 900 --live

See config.py for every tunable.

⚠️ Start in DRY_RUN = True. Monitor for 10+ cycles before --live.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
import urllib.request
import urllib.error
from datetime import datetime, timezone
from pathlib import Path

try:
    import config  # type: ignore[import-not-found]
except Exception:
    sys.path.insert(0, str(Path(__file__).parent))
    import config  # type: ignore[import-not-found]


def log(record: dict) -> None:
    record["ts"] = datetime.now(timezone.utc).isoformat()
    line = json.dumps(record)
    print(line, flush=True)
    if config.LOG_TRADES_TO_FILE:
        with open(config.LOG_FILE, "a") as f:
            f.write(line + "\n")


def fetch_signal(mode: str, coin: str | None = None) -> dict | None:
    if mode == "trending":
        url = f"{config.SIGNAL_FEED_BASE}/v1/trending?limit=5"
    elif mode == "kol-follow":
        url = f"{config.SIGNAL_FEED_BASE}/v1/kol-sentiment"
        if coin:
            url += f"?coin={coin}"
    elif mode == "funding-fade":
        url = f"{config.SIGNAL_FEED_BASE}/v1/funding-extremes?limit=3"
    else:
        raise ValueError(f"unknown mode: {mode}")

    for attempt in range(config.SIGNAL_FEED_RETRIES + 1):
        try:
            req = urllib.request.Request(url, headers={"Accept": "application/json"})
            with urllib.request.urlopen(req, timeout=config.SIGNAL_FEED_TIMEOUT_SEC) as resp:
                return json.loads(resp.read())
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as e:
            log({"event": "signal_feed_error", "mode": mode, "attempt": attempt, "err": str(e)})
            time.sleep(3)
    return None


def pick_trade(mode: str, payload: dict) -> dict | None:
    if mode == "trending":
        signals = payload.get("signals", [])
        if not signals:
            return None
        best = max(signals, key=lambda s: s.get("score", 0) * s.get("confidence", 0))
        if best.get("score", 0) < config.MIN_SCORE:
            return None
        return {
            "coin": best["coin"],
            "direction": best["direction"],
            "score": best.get("score"),
            "confidence": best.get("confidence"),
            "reason": best.get("reason", ""),
        }
    if mode == "kol-follow":
        if payload.get("kol_count", 0) < config.MIN_KOL_COUNT:
            return None
        if payload.get("confidence", 0) < config.MIN_CONFIDENCE_KOL:
            return None
        return {
            "coin": payload["coin"],
            "direction": payload["direction"],
            "score": payload.get("bullish_pct") if payload["direction"] == "long" else payload.get("bearish_pct"),
            "confidence": payload["confidence"],
            "reason": f"KOL consensus {payload.get('bullish_pct', 0):.0%} bullish",
        }
    if mode == "funding-fade":
        longed = payload.get("most_longed") or {}
        shorted = payload.get("most_shorted") or {}
        # Pick whichever extreme is more severe
        longed_abs = abs(longed.get("funding_8h", 0))
        shorted_abs = abs(shorted.get("funding_8h", 0))
        if max(longed_abs, shorted_abs) < config.FUNDING_EXTREME_ABS:
            return None
        target = longed if longed_abs >= shorted_abs else shorted
        return {
            "coin": target["coin"],
            "direction": target["action"],
            "score": longed_abs if target is longed else shorted_abs,
            "confidence": 0.6,
            "reason": f"funding_8h={target['funding_8h']:+.4f}",
        }
    return None


def hl_quickstart_status() -> str:
    try:
        out = subprocess.check_output(["hyperliquid-plugin", "quickstart"], text=True, timeout=15)
        data = json.loads(out)
        return data.get("status", "unknown")
    except Exception as e:
        log({"event": "hl_quickstart_failed", "err": str(e)})
        return "unknown"


def mark_price(coin: str) -> float | None:
    try:
        out = subprocess.check_output(["hyperliquid-plugin", "prices", "--coin", coin], text=True, timeout=15)
        data = json.loads(out)
        return float(data.get("mark_px"))
    except Exception:
        return None


def leverage_cap(mode: str) -> int:
    return {
        "trending": config.MAX_LEVERAGE_TRENDING,
        "kol-follow": config.MAX_LEVERAGE_KOL,
        "funding-fade": config.MAX_LEVERAGE_FUNDING,
    }.get(mode, 3)


def compute_bracket(side: str, mark: float) -> tuple[float, float]:
    if side == "buy":
        sl = mark * (1 - config.SL_PCT)
        tp = mark * (1 + config.TP_PCT)
    else:
        sl = mark * (1 + config.SL_PCT)
        tp = mark * (1 - config.TP_PCT)
    return sl, tp


def fire_trade(trade: dict, mode: str, size_usd: float, live: bool) -> None:
    if config.PAUSED:
        log({"event": "paused", "mode": mode})
        return
    side = "buy" if trade["direction"] == "long" else "sell"
    lev = min(leverage_cap(mode), config.MAX_LEVERAGE_ABSOLUTE)
    mark = mark_price(trade["coin"])
    if mark is None:
        log({"event": "mark_price_unavailable", "coin": trade["coin"]})
        return
    size_tokens = round((size_usd * lev) / mark, 6)
    sl, tp = compute_bracket(side, mark)
    log({"event": "preparing_trade", "mode": mode, "trade": trade, "side": side, "leverage": lev,
         "size_usd": size_usd, "size_tokens": size_tokens, "mark": mark, "sl": sl, "tp": tp,
         "live": live and not config.DRY_RUN})
    confirm = ["--confirm"] if (live and not config.DRY_RUN) else []
    order_cmd = [
        "hyperliquid-plugin", "order",
        "--coin", trade["coin"],
        "--side", side,
        "--size", str(size_tokens),
        "--leverage", str(lev),
        "--strategy-id", "otto-alpha-sniper",
    ] + confirm
    try:
        subprocess.run(order_cmd, check=True, timeout=45)
    except subprocess.CalledProcessError as e:
        log({"event": "order_failed", "err": str(e)})
        return
    if live and not config.DRY_RUN:
        bracket_cmd = [
            "hyperliquid-plugin", "tpsl",
            "--coin", trade["coin"],
            "--sl-px", f"{sl:.6f}",
            "--tp-px", f"{tp:.6f}",
            "--strategy-id", "otto-alpha-sniper",
            "--confirm",
        ]
        try:
            subprocess.run(bracket_cmd, check=True, timeout=30)
        except subprocess.CalledProcessError as e:
            log({"event": "bracket_failed", "err": str(e)})


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["trending", "kol-follow", "funding-fade"], required=True)
    parser.add_argument("--interval", type=int, default=300, help="seconds between cycles")
    parser.add_argument("--size-usd", type=float, default=config.DEFAULT_SIZE_USD)
    parser.add_argument("--coin", type=str, default=None, help="override for kol-follow mode")
    parser.add_argument("--live", action="store_true", help="submit real orders (still gated by config.DRY_RUN)")
    parser.add_argument("--once", action="store_true", help="run one cycle and exit")
    args = parser.parse_args()

    log({"event": "bot_start", "mode": args.mode, "interval": args.interval, "live": args.live,
         "dry_run": config.DRY_RUN, "paused": config.PAUSED})

    while True:
        status = hl_quickstart_status()
        if status != "ready":
            log({"event": "hl_not_ready", "status": status})
        else:
            payload = fetch_signal(args.mode, args.coin)
            if payload is None:
                log({"event": "no_signal_feed"})
            else:
                trade = pick_trade(args.mode, payload)
                if trade is None:
                    log({"event": "no_trade_this_cycle", "mode": args.mode})
                else:
                    fire_trade(trade, args.mode, args.size_usd, args.live)

        if args.once:
            break
        time.sleep(args.interval)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        log({"event": "bot_stop", "reason": "keyboard_interrupt"})
