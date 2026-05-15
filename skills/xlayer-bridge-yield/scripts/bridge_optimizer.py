#!/usr/bin/env python3
"""Bridge route optimizer for xlayer-bridge-yield plugin.

Reads JSON quote data from onchainos swap quote (cross-chain mode)
and ranks routes by composite score = w_fee * fee_score + w_time * time_score + w_slip * slip_score.

Usage:
    python3 bridge_optimizer.py --quotes '<JSON_ARRAY>'
    python3 bridge_optimizer.py --file quotes.json

Output: JSON array of ranked routes with scores.
"""

import argparse
import json
import sys

# Scoring weights (sum = 1.0)
W_FEE = 0.50
W_TIME = 0.30
W_SLIPPAGE = 0.20


def normalize(values, lower_is_better=True):
    """Min-max normalize a list of numbers to [0, 10]."""
    if not values:
        return []
    mn, mx = min(values), max(values)
    if mn == mx:
        return [10.0] * len(values)
    if lower_is_better:
        return [10.0 * (mx - v) / (mx - mn) for v in values]
    return [10.0 * (v - mn) / (mx - mn) for v in values]


def extract_route_fields(route):
    """Extract fee, time, slippage from a route object.

    Handles multiple possible JSON shapes from onchainos swap quote.
    """
    fee = 0.0
    est_time = 0.0
    slippage = 0.0

    # Fee: try several possible field names
    for key in ("bridgeFee", "fee", "gasFee", "totalFee", "estimatedFee"):
        if key in route:
            try:
                fee = float(route[key])
            except (ValueError, TypeError):
                pass
            break

    # Estimated time in minutes
    for key in ("estimatedTime", "time", "bridgeTime", "eta"):
        if key in route:
            try:
                raw = float(route[key])
                # If value looks like seconds (> 300), convert to minutes
                est_time = raw / 60.0 if raw > 300 else raw
            except (ValueError, TypeError):
                pass
            break

    # Slippage percentage
    for key in ("slippage", "priceImpact", "estimatedSlippage"):
        if key in route:
            try:
                slippage = abs(float(route[key]))
            except (ValueError, TypeError):
                pass
            break

    return fee, est_time, slippage


def rank_routes(quotes):
    """Rank bridge routes by composite score.

    Args:
        quotes: list of route dicts from onchainos swap quote

    Returns:
        list of dicts with original fields + score, sorted best-first
    """
    if not quotes:
        return []

    fees = []
    times = []
    slippages = []

    for q in quotes:
        f, t, s = extract_route_fields(q)
        fees.append(f)
        times.append(t)
        slippages.append(s)

    fee_scores = normalize(fees, lower_is_better=True)
    time_scores = normalize(times, lower_is_better=True)
    slip_scores = normalize(slippages, lower_is_better=True)

    results = []
    for i, q in enumerate(quotes):
        composite = (
            W_FEE * fee_scores[i]
            + W_TIME * time_scores[i]
            + W_SLIPPAGE * slip_scores[i]
        )
        results.append({
            "route": q.get("routeName", q.get("bridge", q.get("protocol", f"Route-{i+1}"))),
            "fee_usd": round(fees[i], 4),
            "est_time_min": round(times[i], 1),
            "slippage_pct": round(slippages[i], 4),
            "score": round(composite, 2),
            "raw": q,
        })

    results.sort(key=lambda r: r["score"], reverse=True)
    return results


def main():
    parser = argparse.ArgumentParser(description="Rank bridge routes by cost/speed/slippage")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--quotes", type=str, help="JSON string of quote array")
    group.add_argument("--file", type=str, help="Path to JSON file with quotes")
    args = parser.parse_args()

    if args.file:
        with open(args.file, "r", encoding="utf-8") as f:
            data = json.load(f)
    else:
        data = json.loads(args.quotes)

    # Accept both single route and array
    if isinstance(data, dict):
        if "routes" in data:
            routes = data["routes"]
        elif "data" in data:
            routes = data["data"] if isinstance(data["data"], list) else [data["data"]]
        else:
            routes = [data]
    else:
        routes = data

    ranked = rank_routes(routes)

    print(json.dumps(ranked, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
