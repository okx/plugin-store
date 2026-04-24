#!/usr/bin/env python3
"""
pm-5m-momentum-sniper signal engine.

Fetches 1-minute klines from Binance public API, computes technical indicators
(RSI, EMA crossover, momentum, volume trend), and outputs a directional signal
for Polymarket 5-minute crypto prediction markets.

Usage:
    pm-5m-signal --coin BTC
    pm-5m-signal --coin ETH --period 15
    pm-5m-signal --coin SOL --format json
"""

import argparse
import json
import sys
import urllib.request
import urllib.error
from datetime import datetime, timezone

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

BINANCE_KLINE_URL = "https://api.binance.com/api/v3/klines"

COIN_TO_SYMBOL = {
    "BTC": "BTCUSDT",
    "ETH": "ETHUSDT",
    "SOL": "SOLUSDT",
    "BNB": "BNBUSDT",
    "XRP": "XRPUSDT",
    "DOGE": "DOGEUSDT",
}

# ---------------------------------------------------------------------------
# Data fetching
# ---------------------------------------------------------------------------

def fetch_klines(symbol: str, interval: str = "1m", limit: int = 30) -> list[dict]:
    """Fetch klines from Binance public API. No API key required."""
    url = f"{BINANCE_KLINE_URL}?symbol={symbol}&interval={interval}&limit={limit}"
    req = urllib.request.Request(url, headers={"User-Agent": "pm-5m-signal/1.0"})
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            raw = json.loads(resp.read().decode())
    except (urllib.error.URLError, urllib.error.HTTPError) as e:
        return {"error": f"Failed to fetch klines: {e}"}

    klines = []
    for k in raw:
        klines.append({
            "open_time": k[0],
            "open": float(k[1]),
            "high": float(k[2]),
            "low": float(k[3]),
            "close": float(k[4]),
            "volume": float(k[5]),
            "close_time": k[6],
        })
    return klines


# ---------------------------------------------------------------------------
# Technical indicators
# ---------------------------------------------------------------------------

def calc_rsi(closes: list[float], period: int = 14) -> float:
    """Relative Strength Index. Returns 0-100."""
    if len(closes) < period + 1:
        return 50.0  # neutral fallback

    gains = []
    losses = []
    for i in range(1, len(closes)):
        delta = closes[i] - closes[i - 1]
        gains.append(max(delta, 0))
        losses.append(max(-delta, 0))

    avg_gain = sum(gains[:period]) / period
    avg_loss = sum(losses[:period]) / period

    for i in range(period, len(gains)):
        avg_gain = (avg_gain * (period - 1) + gains[i]) / period
        avg_loss = (avg_loss * (period - 1) + losses[i]) / period

    if avg_loss == 0:
        return 100.0
    rs = avg_gain / avg_loss
    return 100.0 - (100.0 / (1.0 + rs))


def calc_ema(values: list[float], period: int) -> list[float]:
    """Exponential Moving Average."""
    if not values:
        return []
    multiplier = 2.0 / (period + 1)
    ema = [values[0]]
    for i in range(1, len(values)):
        ema.append(values[i] * multiplier + ema[-1] * (1 - multiplier))
    return ema


def calc_momentum(closes: list[float], period: int = 5) -> float:
    """Price change rate over last N candles, as percentage."""
    if len(closes) < period + 1:
        return 0.0
    old = closes[-(period + 1)]
    if old == 0:
        return 0.0
    return ((closes[-1] - old) / old) * 100.0


def calc_volume_trend(klines: list[dict], period: int = 5) -> float:
    """Compare recent volume to earlier volume. >1 means increasing."""
    if len(klines) < period * 2:
        return 1.0
    recent_vol = sum(k["volume"] for k in klines[-period:])
    earlier_vol = sum(k["volume"] for k in klines[-(period * 2):-period])
    if earlier_vol == 0:
        return 1.0
    return recent_vol / earlier_vol


def calc_kelly_stake(
    confidence: float,
    token_price: float,
    budget: float,
    min_stake: float = 2.0,
    max_fraction: float = 0.25,
) -> dict:
    """
    Half-Kelly position sizing.

    confidence: estimated win probability (0-1) from signal engine
    token_price: Polymarket outcome token price (0-1)
    budget: available session budget in USDC
    min_stake: minimum bet size
    max_fraction: max fraction of budget per round (safety cap)

    Returns dict with stake amount and reasoning.
    """
    if token_price <= 0 or token_price >= 1 or confidence <= 0:
        return {"stake": min_stake, "kelly_raw": 0, "kelly_half": 0, "reason": "invalid inputs, using minimum"}

    # Payout odds: if you pay `token_price` and win, you get $1
    # Net odds (b) = (1 - token_price) / token_price
    b = (1.0 - token_price) / token_price

    # Kelly fraction: f* = (bp - q) / b
    # where p = win probability, q = 1 - p
    p = confidence
    q = 1.0 - p
    kelly_raw = (b * p - q) / b if b > 0 else 0

    if kelly_raw <= 0:
        return {
            "stake": 0,
            "kelly_raw": round(kelly_raw, 4),
            "kelly_half": 0,
            "reason": f"negative edge (kelly={kelly_raw:.3f}), skip",
        }

    # Half-Kelly for safety (reduces variance significantly)
    kelly_half = kelly_raw / 2.0

    # Cap at max_fraction of budget
    fraction = min(kelly_half, max_fraction)
    stake = budget * fraction

    # Enforce min/max
    stake = max(stake, min_stake)
    stake = min(stake, budget * max_fraction)

    # Round to 2 decimals
    stake = round(stake, 2)

    return {
        "stake": stake,
        "kelly_raw": round(kelly_raw, 4),
        "kelly_half": round(kelly_half, 4),
        "fraction": round(fraction, 4),
        "reason": f"kelly={kelly_raw:.3f}, half-kelly={kelly_half:.3f}, fraction={fraction:.1%} of ${budget:.0f}",
    }


# ---------------------------------------------------------------------------
# Signal generation
# ---------------------------------------------------------------------------

def generate_signal(coin: str, period: int = 15, budget: float = 50.0, token_price: float = 0.0) -> dict:
    """
    Generate a directional signal for the given coin.

    Returns dict with:
        direction: "up" | "down" | "skip"
        confidence: 0.0 - 1.0
        indicators: { rsi, ema_cross, momentum_pct, volume_ratio }
        reasoning: human-readable explanation
    """
    symbol = COIN_TO_SYMBOL.get(coin.upper())
    if not symbol:
        return {
            "direction": "skip",
            "confidence": 0.0,
            "error": f"Unsupported coin: {coin}. Supported: {', '.join(COIN_TO_SYMBOL.keys())}",
        }

    klines = fetch_klines(symbol, interval="1m", limit=period + 15)
    if isinstance(klines, dict) and "error" in klines:
        return {"direction": "skip", "confidence": 0.0, **klines}

    closes = [k["close"] for k in klines]

    # --- Indicators ---
    rsi = calc_rsi(closes, period=14)
    ema_fast = calc_ema(closes, period=5)
    ema_slow = calc_ema(closes, period=15)
    momentum_pct = calc_momentum(closes, period=5)
    volume_ratio = calc_volume_trend(klines, period=5)

    # EMA crossover: positive = fast above slow (bullish)
    ema_cross = ema_fast[-1] - ema_slow[-1] if ema_fast and ema_slow else 0.0
    # Normalize to basis points for readability
    ema_cross_bps = (ema_cross / closes[-1] * 10000) if closes[-1] else 0.0

    # --- Scoring ---
    score = 0.0
    reasons = []

    # RSI component: -2 to +2
    if rsi > 65:
        score += min((rsi - 50) / 25, 2.0)
        reasons.append(f"RSI {rsi:.1f} bullish")
    elif rsi < 35:
        score -= min((50 - rsi) / 25, 2.0)
        reasons.append(f"RSI {rsi:.1f} bearish")
    else:
        reasons.append(f"RSI {rsi:.1f} neutral")

    # EMA crossover component: -2 to +2
    if ema_cross_bps > 5:
        s = min(ema_cross_bps / 20, 2.0)
        score += s
        reasons.append(f"EMA5>EMA15 by {ema_cross_bps:.1f}bps bullish")
    elif ema_cross_bps < -5:
        s = min(-ema_cross_bps / 20, 2.0)
        score -= s
        reasons.append(f"EMA5<EMA15 by {-ema_cross_bps:.1f}bps bearish")
    else:
        reasons.append("EMA flat")

    # Momentum component: -2 to +2
    if momentum_pct > 0.05:
        s = min(momentum_pct / 0.3, 2.0)
        score += s
        reasons.append(f"Momentum +{momentum_pct:.3f}% bullish")
    elif momentum_pct < -0.05:
        s = min(-momentum_pct / 0.3, 2.0)
        score -= s
        reasons.append(f"Momentum {momentum_pct:.3f}% bearish")
    else:
        reasons.append("Momentum flat")

    # Volume confirmation: amplify signal if volume supports direction
    if volume_ratio > 1.3 and abs(score) > 1:
        score *= 1.2
        reasons.append(f"Volume {volume_ratio:.1f}x confirms trend")
    elif volume_ratio < 0.7 and abs(score) > 1:
        score *= 0.8
        reasons.append(f"Volume {volume_ratio:.1f}x weakens signal")

    # --- Decision ---
    max_score = 6.0  # theoretical max after volume amplification
    confidence = min(abs(score) / max_score, 1.0)

    if score > 1.0:
        direction = "up"
    elif score < -1.0:
        direction = "down"
    else:
        direction = "skip"
        reasons.append("Score too weak, no clear direction")

    # --- Kelly position sizing ---
    if direction != "skip" and token_price > 0:
        kelly = calc_kelly_stake(confidence, token_price, budget)
    elif direction != "skip":
        # No token_price provided, estimate with default 0.50
        kelly = calc_kelly_stake(confidence, 0.50, budget)
    else:
        kelly = {"stake": 0, "kelly_raw": 0, "kelly_half": 0, "reason": "no trade"}

    return {
        "coin": coin.upper(),
        "direction": direction,
        "confidence": round(confidence, 3),
        "score": round(score, 2),
        "indicators": {
            "rsi": round(rsi, 2),
            "ema_cross_bps": round(ema_cross_bps, 2),
            "momentum_pct": round(momentum_pct, 4),
            "volume_ratio": round(volume_ratio, 2),
            "price": closes[-1],
        },
        "sizing": kelly,
        "reasoning": "; ".join(reasons),
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        prog="pm-5m-signal",
        description="Momentum signal engine for Polymarket 5-minute crypto markets",
    )
    parser.add_argument("--coin", required=True, help="Coin symbol: BTC, ETH, SOL, etc.")
    parser.add_argument("--period", type=int, default=15, help="Number of 1m candles to analyze (default: 15)")
    parser.add_argument("--budget", type=float, default=50.0, help="Session budget in USDC for position sizing (default: 50)")
    parser.add_argument("--token-price", type=float, default=0.0, help="Current Polymarket token price for Kelly sizing (0-1)")
    parser.add_argument("--format", choices=["json", "text"], default="text", help="Output format")
    args = parser.parse_args()

    result = generate_signal(args.coin, args.period, args.budget, args.token_price)

    if args.format == "json":
        print(json.dumps(result, indent=2))
    else:
        if "error" in result:
            print(f"ERROR: {result['error']}")
            sys.exit(1)

        ind = result.get("indicators", {})
        direction = result["direction"].upper()
        conf_pct = result["confidence"] * 100

        arrow = {"UP": "▲", "DOWN": "▼", "SKIP": "—"}.get(direction, "?")

        print(f"{'='*50}")
        print(f"  {result['coin']} 5M Signal: {arrow} {direction} ({conf_pct:.0f}% confidence)")
        print(f"{'='*50}")
        print(f"  Price:      ${ind.get('price', 0):,.2f}")
        print(f"  RSI(14):    {ind.get('rsi', 0):.1f}")
        print(f"  EMA cross:  {ind.get('ema_cross_bps', 0):+.1f} bps")
        print(f"  Momentum:   {ind.get('momentum_pct', 0):+.4f}%")
        print(f"  Volume:     {ind.get('volume_ratio', 0):.1f}x")
        print(f"  Score:      {result['score']:+.2f}")
        print(f"{'='*50}")
        print(f"  {result['reasoning']}")
        print()

        if direction == "SKIP":
            print("  >> No trade recommended this round")
        else:
            sizing = result.get("sizing", {})
            stake = sizing.get("stake", 0)
            kelly_r = sizing.get("kelly_raw", 0)
            kelly_h = sizing.get("kelly_half", 0)
            print(f"  >> Recommend: BET {direction}")
            if stake > 0:
                print(f"  >> Suggested stake: ${stake:.2f} (half-kelly={kelly_h:.3f}, full-kelly={kelly_r:.3f})")
            else:
                print(f"  >> Stake: $0 — {sizing.get('reason', 'no edge')}")


if __name__ == "__main__":
    main()
