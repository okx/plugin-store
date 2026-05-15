#!/usr/bin/env python3
"""Yield pool scanner for xlayer-bridge-yield plugin.

Processes token and market data from onchainos to rank DeFi yield pools
on X Layer by a risk-adjusted score.

Usage:
    python3 yield_scanner.py --tokens '<JSON>' --market '<JSON>' --security '<JSON>'
    python3 yield_scanner.py --file pool_data.json

Output: JSON array of ranked pools with risk labels.
"""

import argparse
import json
import sys
from datetime import datetime, timezone

# Minimum thresholds (defaults, can be overridden)
MIN_TVL = 50_000       # USD
MIN_APY = 1.0          # percent
MIN_AGE_DAYS = 30

# Risk scoring weights
W_APY = 0.35
W_TVL = 0.30
W_SECURITY = 0.20
W_AGE = 0.15


def classify_risk(security_score, tvl, age_days):
    """Classify pool risk as LOW, MEDIUM, or HIGH."""
    if security_score <= 2 or tvl < 10_000 or age_days < 7:
        return "HIGH"
    if security_score <= 5 or tvl < MIN_TVL or age_days < MIN_AGE_DAYS:
        return "MEDIUM"
    return "LOW"


def normalize(values, lower_is_better=False):
    """Min-max normalize to [0, 10]."""
    if not values:
        return []
    mn, mx = min(values), max(values)
    if mn == mx:
        return [5.0] * len(values)
    if lower_is_better:
        return [10.0 * (mx - v) / (mx - mn) for v in values]
    return [10.0 * (v - mn) / (mx - mn) for v in values]


def parse_pool_data(tokens_data, market_data, security_data):
    """Merge token, market, and security data into unified pool records."""
    pools = []

    # Build lookup maps
    market_map = {}
    if isinstance(market_data, list):
        for m in market_data:
            addr = m.get("address", m.get("tokenAddress", "")).lower()
            if addr:
                market_map[addr] = m
    elif isinstance(market_data, dict):
        addr = market_data.get("address", "").lower()
        if addr:
            market_map[addr] = market_data

    security_map = {}
    if isinstance(security_data, list):
        for s in security_data:
            addr = s.get("address", s.get("tokenAddress", "")).lower()
            if addr:
                security_map[addr] = s
    elif isinstance(security_data, dict):
        addr = security_data.get("address", "").lower()
        if addr:
            security_map[addr] = security_data

    # Process tokens
    token_list = tokens_data if isinstance(tokens_data, list) else [tokens_data]

    for token in token_list:
        addr = token.get("address", token.get("tokenAddress", "")).lower()
        name = token.get("name", token.get("tokenName", "Unknown"))
        symbol = token.get("symbol", token.get("tokenSymbol", "???"))

        # Market data
        mkt = market_map.get(addr, {})
        apy = 0.0
        tvl = 0.0
        for apy_key in ("apy", "APY", "yield", "annualReturn"):
            if apy_key in mkt:
                try:
                    apy = float(mkt[apy_key])
                except (ValueError, TypeError):
                    pass
                break
        for tvl_key in ("tvl", "TVL", "totalValueLocked", "liquidity"):
            if tvl_key in mkt:
                try:
                    tvl = float(mkt[tvl_key])
                except (ValueError, TypeError):
                    pass
                break

        # Security data
        sec = security_map.get(addr, {})
        sec_score = 5  # default: neutral
        for sec_key in ("riskScore", "score", "securityScore", "riskLevel"):
            if sec_key in sec:
                raw = sec[sec_key]
                if isinstance(raw, str):
                    risk_map = {"low": 8, "medium": 5, "high": 2, "critical": 0}
                    sec_score = risk_map.get(raw.lower(), 5)
                else:
                    try:
                        sec_score = int(raw)
                    except (ValueError, TypeError):
                        pass
                break

        # Pool age
        age_days = 365  # default: assume established
        for age_key in ("createdAt", "launchDate", "deployedAt"):
            if age_key in token:
                try:
                    ts = token[age_key]
                    if isinstance(ts, (int, float)):
                        created = datetime.fromtimestamp(ts, tz=timezone.utc)
                    else:
                        created = datetime.fromisoformat(str(ts).replace("Z", "+00:00"))
                    age_days = (datetime.now(timezone.utc) - created).days
                except (ValueError, TypeError, OSError):
                    pass
                break

        pool = {
            "address": addr,
            "name": name,
            "symbol": symbol,
            "apy": apy,
            "tvl": tvl,
            "security_score": sec_score,
            "age_days": age_days,
            "risk": classify_risk(sec_score, tvl, age_days),
        }
        pools.append(pool)

    return pools


def rank_pools(pools, min_tvl=MIN_TVL, min_apy=MIN_APY):
    """Filter and rank pools by risk-adjusted composite score."""
    # Filter
    filtered = [
        p for p in pools
        if p["tvl"] >= min_tvl
        and p["apy"] >= min_apy
        and p["risk"] != "CRITICAL"
    ]

    if not filtered:
        return []

    apys = [p["apy"] for p in filtered]
    tvls = [p["tvl"] for p in filtered]
    secs = [p["security_score"] for p in filtered]
    ages = [p["age_days"] for p in filtered]

    apy_scores = normalize(apys, lower_is_better=False)
    tvl_scores = normalize(tvls, lower_is_better=False)
    sec_scores = normalize(secs, lower_is_better=False)
    age_scores = normalize(ages, lower_is_better=False)

    results = []
    for i, p in enumerate(filtered):
        composite = (
            W_APY * apy_scores[i]
            + W_TVL * tvl_scores[i]
            + W_SECURITY * sec_scores[i]
            + W_AGE * age_scores[i]
        )
        results.append({
            "pool": f"{p['symbol']} ({p['name']})",
            "address": p["address"],
            "apy_pct": round(p["apy"], 2),
            "tvl_usd": round(p["tvl"], 0),
            "risk": p["risk"],
            "security_score": p["security_score"],
            "age_days": p["age_days"],
            "score": round(composite, 2),
        })

    results.sort(key=lambda r: r["score"], reverse=True)
    return results


def main():
    parser = argparse.ArgumentParser(description="Scan and rank X Layer yield pools")
    parser.add_argument("--tokens", type=str, help="JSON token data from onchainos token search")
    parser.add_argument("--market", type=str, default="{}", help="JSON market data from onchainos market price")
    parser.add_argument("--security", type=str, default="{}", help="JSON security data from onchainos security token-scan")
    parser.add_argument("--file", type=str, help="Path to combined JSON file")
    parser.add_argument("--min-tvl", type=float, default=MIN_TVL, help="Minimum TVL filter (USD)")
    parser.add_argument("--min-apy", type=float, default=MIN_APY, help="Minimum APY filter (percent)")
    args = parser.parse_args()

    if args.file:
        with open(args.file, "r", encoding="utf-8") as f:
            data = json.load(f)
        tokens_data = data.get("tokens", [])
        market_data = data.get("market", {})
        security_data = data.get("security", {})
    else:
        if not args.tokens:
            print(json.dumps({"error": "Either --tokens or --file is required"}))
            return 1
        tokens_data = json.loads(args.tokens)
        market_data = json.loads(args.market)
        security_data = json.loads(args.security)

    pools = parse_pool_data(tokens_data, market_data, security_data)
    ranked = rank_pools(pools, min_tvl=args.min_tvl, min_apy=args.min_apy)

    print(json.dumps(ranked, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
