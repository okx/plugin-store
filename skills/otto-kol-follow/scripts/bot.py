#!/usr/bin/env python3
"""
Otto KOL Follow v0.1 — Optional autonomous poller.

The reactive flow documented in SKILL.md (AI agent fires trades on user intent)
is the primary mode. This bot.py is for advanced users who want a hands-free
background poller that checks the Otto KOL-sentiment feed every N seconds and
fires a trade when cohort size + confidence thresholds are met.

Run:
    python3 bot.py --coin ETH --interval 600

    # Dry-run by default. Pass --live to submit real orders.
    python3 bot.py --coin BTC --interval 900 --live

    # Omit --coin to let the feed pick the top-conviction coin across the cohort.
    python3 bot.py --interval 1200

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


def fetch_sentiment(coin: str | None) -> dict | None:
    url = f"{config.SIGNAL_FEED_BASE}/v1/kol-sentiment"
    if coin:
        url += f"?coin={coin}"
    for attempt in range(config.SIGNAL_FEED_RETRIES + 1):
        try:
            req = urllib.request.Request(url, headers={"Accept": "application/json"})
            with urllib.request.urlopen(req, timeout=config.SIGNAL_FEED_TIMEOUT) as resp:
                return json.loads(resp.read())
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as e:
            log({"event": "signal_feed_error", "attempt": attempt, "err": str(e)})
            time.sleep(3)
    return None


def fresh_enough(payload: dict) -> bool:
    try:
        ts = datetime.fromisoformat(payload["updated_at"].replace("Z", "+00:00"))
        age = (datetime.now(timezone.utc) - ts).total_seconds()
        return age <= config.MAX_SIGNAL_AGE_SEC
    except Exception:
        return False


def pick_trade(payload: dict) -> dict | None:
    if payload.get("status") == "degraded":
        return None
    if payload.get("kol_count", 0) < config.MIN_KOL_COUNT:
        return None
    if payload.get("confidence", 0) < config.MIN_CONFIDENCE_KOL:
        return None
    direction = payload.get("direction")
    if direction not in ("long", "short"):
        return None
    return {
        "coin": payload["coin"],
        "direction": direction,
        "confidence": payload["confidence"],
        "kol_count": payload["kol_count"],
        "bullish_pct": payload.get("bullish_pct"),
        "bearish_pct": payload.get("bearish_pct"),
        "window_hours": payload.get("window_hours"),
    }


def hl_quickstart_status() -> str:
    try:
        out = subprocess.check_output(["hyperliquid-plugin", "quickstart"], text=True, timeout=15)
        return json.loads(out).get("status", "unknown")
    except Exception as e:
        log({"event": "hl_quickstart_failed", "err": str(e)})
        return "unknown"


def mark_price(coin: str) -> float | None:
    try:
        out = subprocess.check_output(["hyperliquid-plugin", "prices", "--coin", coin], text=True, timeout=15)
        return float(json.loads(out).get("mark_px"))
    except Exception:
        return None


def compute_bracket(side: str, mark: float) -> tuple[float, float]:
    if side == "buy":
        sl = mark * (1 - config.SL_PCT)
        tp = mark * (1 + config.TP_PCT)
    else:
        sl = mark * (1 + config.SL_PCT)
        tp = mark * (1 - config.TP_PCT)
    return sl, tp


def fire_trade(trade: dict, size_usd: float, live: bool) -> None:
    if config.PAUSED:
        log({"event": "paused"})
        return
    side = "buy" if trade["direction"] == "long" else "sell"
    lev = min(config.MAX_LEVERAGE_KOL, config.MAX_LEVERAGE_ABSOLUTE)
    mark = mark_price(trade["coin"])
    if mark is None:
        log({"event": "mark_price_unavailable", "coin": trade["coin"]})
        return
    size_tokens = round((size_usd * lev) / mark, 6)
    sl, tp = compute_bracket(side, mark)
    log({"event": "preparing_trade", "trade": trade, "side": side, "leverage": lev,
         "size_usd": size_usd, "size_tokens": size_tokens, "mark": mark, "sl": sl, "tp": tp,
         "live": live and not config.DRY_RUN})
    confirm = ["--confirm"] if (live and not config.DRY_RUN) else []
    order_cmd = [
        "hyperliquid-plugin", "order",
        "--coin", trade["coin"],
        "--side", side,
        "--size", str(size_tokens),
        "--leverage", str(lev),
        "--strategy-id", "otto-kol-follow",
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
            "--strategy-id", "otto-kol-follow",
            "--confirm",
        ]
        try:
            subprocess.run(bracket_cmd, check=True, timeout=30)
        except subprocess.CalledProcessError as e:
            log({"event": "bracket_failed", "err": str(e)})


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--coin", type=str, default=None, help="coin to track; omit for top-conviction cohort pick")
    parser.add_argument("--interval", type=int, default=600, help="seconds between cycles")
    parser.add_argument("--size-usd", type=float, default=config.DEFAULT_SIZE_USD)
    parser.add_argument("--live", action="store_true", help="submit real orders (still gated by config.DRY_RUN)")
    parser.add_argument("--once", action="store_true", help="run one cycle and exit")
    args = parser.parse_args()

    log({"event": "bot_start", "coin": args.coin, "interval": args.interval, "live": args.live,
         "dry_run": config.DRY_RUN, "paused": config.PAUSED})

    while True:
        status = hl_quickstart_status()
        if status != "ready":
            log({"event": "hl_not_ready", "status": status})
        else:
            payload = fetch_sentiment(args.coin)
            if payload is None:
                log({"event": "no_signal_feed"})
            elif not fresh_enough(payload):
                log({"event": "signal_stale", "updated_at": payload.get("updated_at")})
            else:
                trade = pick_trade(payload)
                if trade is None:
                    log({"event": "no_trade_this_cycle", "reason": "cohort/confidence/direction gate",
                         "kol_count": payload.get("kol_count"), "confidence": payload.get("confidence"),
                         "direction": payload.get("direction")})
                else:
                    fire_trade(trade, args.size_usd, args.live)

        if args.once:
            break
        time.sleep(args.interval)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        log({"event": "bot_stop", "reason": "keyboard_interrupt"})
