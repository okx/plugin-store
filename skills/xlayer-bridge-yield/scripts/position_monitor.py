#!/usr/bin/env python3
"""Position monitor for xlayer-bridge-yield plugin.

Tracks yield farming positions on X Layer, calculates P/L,
and generates alerts for depeg events, stop-loss triggers,
and APY changes.

Usage:
    python3 position_monitor.py --balances '<JSON>' --prices '<JSON>'
    python3 position_monitor.py --file positions.json

Output: JSON with position summary and active alerts.
"""

import argparse
import json
import sys

# Alert thresholds (defaults)
DEPEG_THRESHOLD = 0.005     # 0.5% deviation from $1.00
STOP_LOSS_PCT = -5.0        # percent
APY_DROP_THRESHOLD = 2.0    # percentage points

# Known stablecoins (for depeg detection)
STABLECOINS = {
    "usdc", "usdt", "dai", "tusd", "busd", "frax",
    "usd coin", "tether", "dai stablecoin",
}


def is_stablecoin(symbol, name=""):
    """Check if a token is a stablecoin by symbol or name."""
    sym = symbol.lower().strip()
    nm = name.lower().strip()
    return sym in STABLECOINS or any(s in nm for s in STABLECOINS)


def check_depeg(price, threshold=DEPEG_THRESHOLD):
    """Check if a stablecoin has depegged beyond threshold."""
    if price <= 0:
        return True, 1.0
    deviation = abs(price - 1.0)
    return deviation > threshold, deviation


def analyze_positions(balances, prices, stop_loss_pct=None, depeg_threshold=None):
    """Analyze positions and generate alerts.

    Args:
        balances: list of balance records from onchainos portfolio
        prices: list/dict of price records from onchainos market price
        stop_loss_pct: override stop-loss threshold (default: STOP_LOSS_PCT)
        depeg_threshold: override depeg threshold (default: DEPEG_THRESHOLD)

    Returns:
        dict with positions list and alerts list
    """
    sl_pct = stop_loss_pct if stop_loss_pct is not None else STOP_LOSS_PCT
    dp_thresh = depeg_threshold if depeg_threshold is not None else DEPEG_THRESHOLD
    # Build price lookup
    price_map = {}
    if isinstance(prices, list):
        for p in prices:
            addr = p.get("address", p.get("tokenAddress", "")).lower()
            sym = p.get("symbol", p.get("tokenSymbol", "")).lower()
            price_val = 0.0
            for k in ("price", "currentPrice", "usdPrice", "lastPrice"):
                if k in p:
                    try:
                        price_val = float(p[k])
                    except (ValueError, TypeError):
                        pass
                    break
            if addr:
                price_map[addr] = price_val
            if sym:
                price_map[sym] = price_val
    elif isinstance(prices, dict):
        for key, val in prices.items():
            try:
                price_map[key.lower()] = float(val)
            except (ValueError, TypeError):
                if isinstance(val, dict):
                    for k in ("price", "currentPrice", "usdPrice"):
                        if k in val:
                            try:
                                price_map[key.lower()] = float(val[k])
                            except (ValueError, TypeError):
                                pass
                            break

    positions = []
    alerts = []

    # Process balances
    balance_list = balances if isinstance(balances, list) else [balances]

    for bal in balance_list:
        symbol = bal.get("symbol", bal.get("tokenSymbol", "???"))
        name = bal.get("name", bal.get("tokenName", ""))
        addr = bal.get("address", bal.get("tokenAddress", "")).lower()
        amount = 0.0
        for k in ("balance", "amount", "tokenAmount", "holdingAmount"):
            if k in bal:
                try:
                    amount = float(bal[k])
                except (ValueError, TypeError):
                    pass
                break

        # Skip zero/dust balances
        if amount < 0.001:
            continue

        # Get current price
        price = price_map.get(addr, price_map.get(symbol.lower(), 0.0))
        value = amount * price if price > 0 else 0.0

        # Determine status
        status = "HEALTHY"
        is_stable = is_stablecoin(symbol, name)

        if is_stable and price > 0:
            depegged, deviation = check_depeg(price, dp_thresh)
            if depegged:
                status = "DEPEG_WARNING"
                alerts.append({
                    "type": "DEPEG",
                    "severity": "HIGH" if deviation > 0.02 else "MEDIUM",
                    "token": symbol,
                    "message": f"{symbol} price ${price:.4f} deviates {deviation*100:.2f}% from $1.00 peg",
                    "action": "Consider exiting position or monitoring closely",
                })

        # Entry price tracking (from balance metadata if available)
        entry_price = None
        for k in ("entryPrice", "avgCost", "costBasis"):
            if k in bal:
                try:
                    entry_price = float(bal[k])
                except (ValueError, TypeError):
                    pass
                break

        change_pct = None
        change_usd = None
        if entry_price and entry_price > 0 and price > 0:
            change_pct = ((price - entry_price) / entry_price) * 100
            change_usd = (price - entry_price) * amount

            if change_pct <= sl_pct:
                status = "STOP_LOSS"
                alerts.append({
                    "type": "STOP_LOSS",
                    "severity": "CRITICAL",
                    "token": symbol,
                    "message": f"{symbol} position down {change_pct:.1f}% (threshold: {sl_pct}%)",
                    "action": "Consider exiting position to limit losses",
                })

        positions.append({
            "token": symbol,
            "name": name,
            "amount": round(amount, 6),
            "price_usd": round(price, 4) if price > 0 else None,
            "value_usd": round(value, 2) if value > 0 else None,
            "change_pct": round(change_pct, 2) if change_pct is not None else None,
            "change_usd": round(change_usd, 2) if change_usd is not None else None,
            "status": status,
            "is_stablecoin": is_stable,
        })

    # Sort: alerts first, then by value descending
    status_order = {"STOP_LOSS": 0, "DEPEG_WARNING": 1, "HEALTHY": 2}
    positions.sort(key=lambda p: (status_order.get(p["status"], 9), -(p["value_usd"] or 0)))

    total_value = sum(p["value_usd"] for p in positions if p["value_usd"])
    total_change = sum(p["change_usd"] for p in positions if p["change_usd"] is not None)

    return {
        "positions": positions,
        "alerts": alerts,
        "summary": {
            "total_value_usd": round(total_value, 2),
            "total_change_usd": round(total_change, 2) if total_change else None,
            "position_count": len(positions),
            "alert_count": len(alerts),
        },
    }


def main():
    parser = argparse.ArgumentParser(description="Monitor X Layer yield positions")
    parser.add_argument("--balances", type=str, help="JSON balance data from onchainos portfolio")
    parser.add_argument("--prices", type=str, default="{}", help="JSON price data from onchainos market price")
    parser.add_argument("--file", type=str, help="Path to combined JSON file")
    parser.add_argument("--stop-loss", type=float, default=STOP_LOSS_PCT, help="Stop-loss threshold (percent)")
    parser.add_argument("--depeg-threshold", type=float, default=DEPEG_THRESHOLD, help="Depeg alert threshold")
    args = parser.parse_args()

    if args.file:
        with open(args.file, "r", encoding="utf-8") as f:
            data = json.load(f)
        balances = data.get("balances", [])
        prices = data.get("prices", {})
    else:
        if not args.balances:
            print(json.dumps({"error": "Either --balances or --file is required"}))
            return 1
        balances = json.loads(args.balances)
        prices = json.loads(args.prices)

    result = analyze_positions(balances, prices,
                               stop_loss_pct=args.stop_loss,
                               depeg_threshold=args.depeg_threshold)
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
