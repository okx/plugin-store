#!/usr/bin/env python3
"""
Otto Mispricing Assistant v0.1 — Optional scanner (notify-only).

The reactive flow documented in SKILL.md is the primary mode. This bot.py is for
advanced users who want a background watcher that periodically scans Polymarket
for mispricings and logs candidates. **This bot NEVER places orders.** It is
intentionally notify-only — to trade, the user paste a market ID back into the
reactive Skill flow where the Step 7 confirmation protocol enforces safety.

Run:
    python3 bot.py --interval 900 --notify-only      # scan every 15 min, log only

See config.py for every tunable.
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
    if config.LOG_SCANS_TO_FILE:
        with open(config.LOG_FILE, "a") as f:
            f.write(line + "\n")


def _http_json(url: str) -> dict | None:
    for attempt in range(config.SIGNAL_FEED_RETRIES + 1):
        try:
            req = urllib.request.Request(url, headers={"Accept": "application/json"})
            with urllib.request.urlopen(req, timeout=config.SIGNAL_FEED_TIMEOUT) as resp:
                return json.loads(resp.read())
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as e:
            log({"event": "http_error", "url": url, "attempt": attempt, "err": str(e)})
            time.sleep(3)
    return None


def fetch_otto_signals() -> dict:
    return {
        "news": _http_json(f"{config.SIGNAL_FEED_BASE}/v1/news-flash?since_minutes={config.WINDOW_MIN_NEWS}&severity_min=3"),
        "kol": _http_json(f"{config.SIGNAL_FEED_BASE}/v1/kol-sentiment"),
        "funding": _http_json(f"{config.SIGNAL_FEED_BASE}/v1/funding-extremes?limit=5"),
    }


def fetch_polymarket_markets(category: str) -> list[dict]:
    try:
        out = subprocess.check_output(
            ["polymarket-plugin", "list-markets", "--limit", "40", "--category", category],
            text=True, timeout=20,
        )
        data = json.loads(out)
        return data.get("markets", []) if isinstance(data, dict) else data
    except Exception as e:
        log({"event": "polymarket_list_failed", "category": category, "err": str(e)})
        return []


def polymarket_ready() -> bool:
    try:
        out = subprocess.check_output(["polymarket-plugin", "quickstart"], text=True, timeout=15)
        data = json.loads(out)
        return data.get("status") in ("ready", "proxy_ready", "eoa_ready")
    except Exception as e:
        log({"event": "polymarket_quickstart_failed", "err": str(e)})
        return False


def estimate_otto_probability(market: dict, signals: dict) -> tuple[float, float, list[str]]:
    """
    Very conservative v0.1 estimator — combines matched news-flash direction_bias
    and KOL sentiment magnitude. Returns (prob, confidence, matched_signal_tags).

    This is intentionally simple. v0.2 will incorporate funding extremes and a
    learned calibration pass.
    """
    question = (market.get("question") or "").lower()
    tags = []
    conf = 0.0
    delta = 0.0

    news = (signals.get("news") or {}).get("flashes", [])
    for flash in news:
        assets = [a.lower() for a in flash.get("affected_assets", [])]
        for asset in assets:
            if asset and asset in question:
                severity = flash.get("severity", 3) / 5.0
                if flash.get("direction_bias") == "risk-on":
                    delta += 0.05 * severity
                elif flash.get("direction_bias") == "risk-off":
                    delta -= 0.05 * severity
                conf = max(conf, severity)
                tags.append(f"news:{flash.get('id','?')}")

    kol = signals.get("kol") or {}
    kol_coin = (kol.get("coin") or "").lower()
    if kol_coin and kol_coin in question and kol.get("kol_count", 0) >= 30:
        if kol.get("direction") == "long":
            delta += 0.04 * kol.get("confidence", 0)
        elif kol.get("direction") == "short":
            delta -= 0.04 * kol.get("confidence", 0)
        conf = max(conf, kol.get("confidence", 0))
        tags.append(f"kol:{kol_coin}")

    try:
        implied = float(market.get("yes_price", 0.5))
    except (TypeError, ValueError):
        implied = 0.5
    otto_estimate = max(0.02, min(0.98, implied + delta))
    return otto_estimate, conf, tags


def score_candidate(market: dict, signals: dict) -> dict | None:
    try:
        implied = float(market.get("yes_price", 0))
        liquidity = float(market.get("liquidity_usd", 0))
        volume = float(market.get("volume_usd", 0))
    except (TypeError, ValueError):
        return None
    if liquidity < config.MIN_LIQUIDITY_USD or volume < config.MIN_VOLUME_USD:
        return None

    otto_estimate, signal_conf, tags = estimate_otto_probability(market, signals)
    edge = otto_estimate - implied
    abs_edge = abs(edge)
    if abs_edge < config.MIN_EDGE_PCT or signal_conf < 0.2:
        return None
    liq_score = min(liquidity / 10_000, 1.0)
    score = abs_edge * signal_conf * liq_score

    return {
        "market_id": market.get("market_id") or market.get("id"),
        "question": market.get("question"),
        "end_date": market.get("end_date"),
        "implied_prob": implied,
        "otto_estimate": otto_estimate,
        "edge_pct": edge,
        "signal_conf": signal_conf,
        "liquidity_usd": liquidity,
        "volume_usd": volume,
        "mispricing_score": score,
        "side": "yes" if edge > 0 else "no",
        "matched_signals": tags,
    }


def scan_once() -> list[dict]:
    if not polymarket_ready():
        log({"event": "polymarket_not_ready"})
        return []
    signals = fetch_otto_signals()
    if not any(signals.values()):
        log({"event": "no_signals_available"})
        return []
    candidates: list[dict] = []
    for category in config.DEFAULT_CATEGORIES:
        for market in fetch_polymarket_markets(category):
            scored = score_candidate(market, signals)
            if scored:
                scored["category"] = category
                candidates.append(scored)
    candidates.sort(key=lambda c: c["mispricing_score"], reverse=True)
    return candidates[: config.TOP_N_CANDIDATES]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--interval", type=int, default=900, help="seconds between scans")
    parser.add_argument("--notify-only", action="store_true", default=True,
                        help="log candidates only; this bot NEVER trades")
    parser.add_argument("--once", action="store_true")
    args = parser.parse_args()

    log({"event": "scanner_start", "interval": args.interval, "notify_only": True,
         "dry_run": config.DRY_RUN, "paused": config.PAUSED})

    while True:
        if config.PAUSED:
            log({"event": "paused"})
        else:
            candidates = scan_once()
            if not candidates:
                log({"event": "no_candidates_this_cycle"})
            else:
                log({"event": "candidates", "count": len(candidates), "top": candidates})

        if args.once:
            break
        time.sleep(args.interval)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        log({"event": "scanner_stop", "reason": "keyboard_interrupt"})
