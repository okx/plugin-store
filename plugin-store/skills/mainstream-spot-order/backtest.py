#!/usr/bin/env python3
"""Backtest runner. Imports strategy, runs backtest, prints JSON to stdout.

Usage:
    python3 backtest.py --pair SOL
    python3 backtest.py --pair ETH
"""
from __future__ import annotations

import argparse
import json
import os
import sys

import config
import prepare
import strategy

DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")


def main():
    parser = argparse.ArgumentParser(description="Backtest runner")
    parser.add_argument("--pair", default="SOL",
                        help="Trading pair (default: SOL). See: python3 config.py --list")
    args = parser.parse_args()

    config.ACTIVE_PAIR = args.pair.upper()
    p = config.pair()
    symbol = p["base_symbol"].lower()

    base_csv = os.path.join(DATA_DIR, f"{symbol}_15m.csv")
    # For BTC pair, base and overlay are the same file
    if args.pair == "BTC":
        btc_csv = base_csv
    else:
        btc_csv = os.path.join(DATA_DIR, "btc_15m.csv")

    # Load data
    try:
        base = prepare.load_candles(base_csv)
        btc = prepare.load_candles(btc_csv)
    except FileNotFoundError as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)

    print(f"Loaded {len(base)} {p['base_symbol']} bars, {len(btc)} BTC bars", file=sys.stderr)

    # Run backtest
    results = prepare.run_backtest(base, btc, strategy, config)

    if "error" in results:
        print(json.dumps(results, indent=2))
        sys.exit(1)

    # Compute score
    score = prepare.compute_score(results)

    # Output summary (trades and equity_curve trimmed for stdout)
    summary = {k: v for k, v in results.items() if k not in ("trades", "equity_curve")}
    summary["score"] = score
    summary["pair"] = args.pair
    summary["num_bars_aligned"] = results["total_bars"]

    # Save full results to file
    results_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "results")
    os.makedirs(results_dir, exist_ok=True)
    full_path = os.path.join(results_dir, f"latest_{symbol}.json")
    full_results = {**results, "score": score, "pair": args.pair}
    with open(full_path, "w") as f:
        json.dump(full_results, f, indent=2)

    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
