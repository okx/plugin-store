#!/usr/bin/env python3
"""Market Structure Data Fetcher v3.0 — OKX CeFi CLI + OnchainOS.

Usage:
    python3 fetch_market_data.py BTC          # single token
    python3 fetch_market_data.py BTC ETH SOL  # multi-token
    python3 fetch_market_data.py --all         # all supported tokens

Outputs JSON to stdout with all available indicators per token.
Primary: OKX CeFi CLI (`okx market`) for derivatives + price data.
Secondary: Direct HTTP for options chain (gamma wall, skew) + external macro.
External: CoinMetrics (MVRV), alternative.me (F&G), CoinGecko, DefiLlama.

v3.0 changes:
  - Switched from direct OKX HTTP API to `okx` CLI (okx-cex-market skill)
  - Removed Binance fallback (single-source: OKX)
  - Added OI history via `okx market oi-history` with bar-over-bar delta
  - Long/short ratio via `okx market indicator top-long-short`
  - Concurrent CLI calls for speed (ThreadPoolExecutor)
  - Kept direct HTTP for options chain (gamma wall, skew) — too complex for CLI
"""
from __future__ import annotations

import json
import math
import os
import subprocess
import sys
import time
import urllib.request
import urllib.parse
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone

# ── Config ──────────────────────────────────────────────────────────

TOKEN_MAP = {
    "BTC": {
        "okx_swap": "BTC-USDT-SWAP", "okx_spot": "BTC-USDT", "okx_family": "BTC-USD",
        "coingecko": "bitcoin", "tier": 1,
    },
    "ETH": {
        "okx_swap": "ETH-USDT-SWAP", "okx_spot": "ETH-USDT", "okx_family": "ETH-USD",
        "coingecko": "ethereum", "tier": 1,
    },
    "SOL": {
        "okx_swap": "SOL-USDT-SWAP", "okx_spot": "SOL-USDT", "okx_family": "SOL-USD",
        "coingecko": "solana", "tier": 2,
    },
    "BNB": {
        "okx_swap": "BNB-USDT-SWAP", "okx_spot": "BNB-USDT", "okx_family": "",
        "coingecko": "binancecoin", "tier": 2,
    },
    "DOGE": {
        "okx_swap": "DOGE-USDT-SWAP", "okx_spot": "DOGE-USDT", "okx_family": "",
        "coingecko": "dogecoin", "tier": 2,
    },
    "AVAX": {
        "okx_swap": "AVAX-USDT-SWAP", "okx_spot": "AVAX-USDT", "okx_family": "",
        "coingecko": "avalanche-2", "tier": 2,
    },
    "ARB": {
        "okx_swap": "ARB-USDT-SWAP", "okx_spot": "ARB-USDT", "okx_family": "",
        "coingecko": "arbitrum", "tier": 2,
    },
    "XRP": {
        "okx_swap": "XRP-USDT-SWAP", "okx_spot": "XRP-USDT", "okx_family": "",
        "coingecko": "ripple", "tier": 2,
    },
    "LINK": {
        "okx_swap": "LINK-USDT-SWAP", "okx_spot": "LINK-USDT", "okx_family": "",
        "coingecko": "chainlink", "tier": 2,
    },
    "PEPE": {
        "okx_swap": "PEPE-USDT-SWAP", "okx_spot": "PEPE-USDT", "okx_family": "",
        "coingecko": "pepe", "tier": 3,
    },
}

OKX_BASE = "https://www.okx.com/api/v5"       # kept for options chain (gamma wall, skew)
HEADERS = {"User-Agent": "okx-market-analyzer/3.0", "Accept": "application/json"}

# OKX CeFi CLI — installed via `npm install -g @okx_ai/okx-trade-cli`
OKX_CLI = os.environ.get("OKX_CLI_PATH", "/Users/victorlee/.npm-global]/bin/okx")


# ── Helpers ─────────────────────────────────────────────────────────

def safe_float(val, default=0.0) -> float:
    """Safely convert to float; returns default on None, empty string, or bad value."""
    if val is None or val == "":
        return default
    try:
        return float(val)
    except (ValueError, TypeError):
        return default


def fetch(url: str, timeout: int = 12, retries: int = 1) -> dict | list | None:
    """Fetch JSON from URL with retry. Returns parsed data or error dict."""
    for attempt in range(retries + 1):
        try:
            req = urllib.request.Request(url, headers=HEADERS)
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return json.loads(resp.read().decode())
        except Exception as e:
            if attempt < retries:
                time.sleep(0.5 * (attempt + 1))
                continue
            return {"_error": str(e), "_url": url}


def is_error(data) -> bool:
    return data is None or (isinstance(data, dict) and "_error" in data)


def okx_data(resp) -> list:
    """Extract .data array from OKX API response, checking for OKX error codes."""
    if is_error(resp):
        return []
    if not isinstance(resp, dict):
        return []
    # OKX returns {"code": "0", "data": [...]} on success
    # and {"code": "50011", "data": [], "msg": "Rate limit"} on error
    code = resp.get("code", "0")
    if code != "0":
        return []
    return resp.get("data", [])


# ═══════════════════════════════════════════════════════════════════
# OKX CeFi CLI HELPER
# ═══════════════════════════════════════════════════════════════════

def okx_cli(args: list[str], timeout: int = 15) -> list | dict | None:
    """Run `okx <args> --json` and return parsed JSON.

    Returns the parsed JSON on success, None on failure.
    The CLI returns a JSON array or object directly.
    """
    cmd = [OKX_CLI] + args + ["--json"]
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=timeout,
        )
        if result.returncode != 0:
            return None
        return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError, FileNotFoundError):
        return None


# ═══════════════════════════════════════════════════════════════════
# OKX CEFI CLI DATA (PRIMARY)
# ═══════════════════════════════════════════════════════════════════

def okx_funding(inst_id: str) -> dict:
    """Current + next funding rate via `okx market funding-rate`."""
    if not inst_id:
        return {"status": "unavailable"}
    data = okx_cli(["market", "funding-rate", inst_id])
    if not data or not isinstance(data, list) or not data:
        return {"status": "unavailable"}
    r = data[0]
    rate = safe_float(r.get("fundingRate"))
    next_rate = safe_float(r.get("nextFundingRate")) if r.get("nextFundingRate") else None
    return {
        "status": "available",
        "rate": rate,
        "rate_pct": round(rate * 100, 6),
        "rate_annualized_pct": round(rate * 3 * 365 * 100, 2),
        "next_rate": next_rate,
        "next_rate_pct": round(next_rate * 100, 6) if next_rate is not None else None,
        "time": r.get("fundingTime", ""),
        "source": "okx-cli",
    }


def okx_funding_history(inst_id: str, limit: int = 6) -> dict:
    """Recent funding rate history via `okx market funding-rate --history`.

    Returns trend direction and average rate.
    """
    if not inst_id:
        return {"status": "unavailable"}
    data = okx_cli(["market", "funding-rate", inst_id, "--history", "--limit", str(limit)])
    if not data or not isinstance(data, list) or not data:
        return {"status": "unavailable"}
    rates = [safe_float(r.get("fundingRate")) for r in data if isinstance(r, dict)]
    if not rates:
        return {"status": "unavailable"}

    avg = sum(rates) / len(rates)
    mid = len(rates) // 2
    recent_avg = sum(rates[:mid]) / max(mid, 1)
    older_avg = sum(rates[mid:]) / max(len(rates) - mid, 1)
    if recent_avg > older_avg * 1.2:
        trend = "increasing"
    elif recent_avg < older_avg * 0.8:
        trend = "decreasing"
    else:
        trend = "stable"

    return {
        "status": "available",
        "rates": [round(r * 100, 6) for r in rates],
        "avg_rate_pct": round(avg * 100, 6),
        "avg_annualized_pct": round(avg * 3 * 365 * 100, 2),
        "trend": trend,
        "periods": len(rates),
        "source": "okx-cli",
    }


def okx_open_interest(inst_id: str) -> dict:
    """Open interest via `okx market open-interest`."""
    if not inst_id:
        return {"status": "unavailable"}
    data = okx_cli(["market", "open-interest", "--instType", "SWAP", "--instId", inst_id])
    if not data or not isinstance(data, list) or not data:
        return {"status": "unavailable"}
    r = data[0]
    return {
        "status": "available",
        "oi": safe_float(r.get("oi")),
        "oi_currency": safe_float(r.get("oiCcy")),
        "oi_usd": safe_float(r.get("oiUsd")),
        "source": "okx-cli",
    }


def okx_oi_history(inst_id: str) -> dict:
    """OI history with bar-over-bar delta via `okx market oi-history`.

    The CLI returns rows with oiCcy, oiUsd, oiDeltaPct per bar.
    """
    if not inst_id:
        return {"status": "unavailable"}
    data = okx_cli(["market", "oi-history", inst_id, "--bar", "1H", "--limit", "24"])
    if not data or not isinstance(data, list) or not data:
        return {"status": "unavailable"}

    # CLI returns [{bar, instId, rows: [{oiCcy, oiCont, oiDeltaPct, oiDeltaUsd, oiUsd, ts}]}]
    entry = data[0] if data else {}
    rows = entry.get("rows", [])
    if not rows:
        return {"status": "unavailable"}

    latest = rows[0]
    latest_oi = safe_float(latest.get("oiCcy"))
    latest_oi_usd = safe_float(latest.get("oiUsd"))

    # Sum up delta over 24 bars for 24h aggregate delta
    deltas = [safe_float(r.get("oiDeltaPct")) for r in rows]
    oi_delta_1d_pct = sum(deltas) if deltas else 0
    oi_delta_step_pct = safe_float(latest.get("oiDeltaPct"))

    return {
        "status": "available",
        "latest_oi": latest_oi,
        "latest_oi_usd": latest_oi_usd,
        "oi_delta_1d_pct": round(oi_delta_1d_pct, 2),
        "oi_delta_step_pct": round(oi_delta_step_pct, 2),
        "data_points": len(rows),
        "source": "okx-cli",
    }


def okx_ticker(inst_id: str) -> dict:
    """24h ticker via `okx market ticker` CLI."""
    if not inst_id:
        return {"status": "unavailable"}
    data = okx_cli(["market", "ticker", inst_id])
    if not data or not isinstance(data, list) or not data:
        return {"status": "unavailable"}
    r = data[0]
    last = safe_float(r.get("last"))
    open24 = safe_float(r.get("open24h"))
    change_pct = ((last - open24) / open24 * 100) if open24 > 0 else 0
    return {
        "status": "available",
        "price": last,
        "open_24h": open24,
        "high_24h": safe_float(r.get("high24h")),
        "low_24h": safe_float(r.get("low24h")),
        "volume_24h_base": safe_float(r.get("vol24h")),
        "volume_24h_quote": safe_float(r.get("volCcy24h")),
        "price_change_pct": round(change_pct, 2),
        "source": "okx-cli",
    }


def okx_ticker_fast(inst_id: str) -> dict:
    """24h ticker via direct HTTP — 5-8x faster than CLI for high-frequency polling."""
    if not inst_id:
        return {"status": "unavailable"}
    resp = fetch(f"{OKX_BASE}/market/ticker?instId={inst_id}", timeout=5)
    items = okx_data(resp)
    if not items:
        return {"status": "unavailable"}
    r = items[0]
    last = safe_float(r.get("last"))
    open24 = safe_float(r.get("open24h"))
    change_pct = ((last - open24) / open24 * 100) if open24 > 0 else 0
    return {
        "status": "available",
        "price": last,
        "open_24h": open24,
        "high_24h": safe_float(r.get("high24h")),
        "low_24h": safe_float(r.get("low24h")),
        "volume_24h_base": safe_float(r.get("vol24h")),
        "volume_24h_quote": safe_float(r.get("volCcy24h")),
        "price_change_pct": round(change_pct, 2),
        "source": "okx",
    }


def okx_funding_fast(inst_id: str) -> dict:
    """Funding rate via direct HTTP — faster than CLI for concurrent fetching."""
    if not inst_id:
        return {"status": "unavailable"}
    resp = fetch(f"{OKX_BASE}/public/funding-rate?instId={inst_id}", timeout=5)
    items = okx_data(resp)
    if not items:
        return {"status": "unavailable"}
    r = items[0]
    rate = safe_float(r.get("fundingRate"))
    next_rate = safe_float(r.get("nextFundingRate")) if r.get("nextFundingRate") else None
    return {
        "status": "available",
        "rate": rate,
        "rate_pct": round(rate * 100, 6),
        "rate_annualized_pct": round(rate * 3 * 365 * 100, 2),
        "next_rate": next_rate,
        "next_rate_pct": round(next_rate * 100, 6) if next_rate is not None else None,
        "time": r.get("fundingTime", ""),
        "source": "okx",
    }


def okx_oi_fast(inst_id: str) -> dict:
    """Open interest via direct HTTP — faster than CLI."""
    if not inst_id:
        return {"status": "unavailable"}
    resp = fetch(f"{OKX_BASE}/public/open-interest?instType=SWAP&instId={inst_id}", timeout=5)
    items = okx_data(resp)
    if not items:
        return {"status": "unavailable"}
    r = items[0]
    return {
        "status": "available",
        "oi": safe_float(r.get("oi")),
        "oi_currency": safe_float(r.get("oiCcy")),
        "oi_usd": safe_float(r.get("oiUsd")),
        "source": "okx",
    }


def okx_candle_history(inst_id: str, bar: str = "1H", limit: int = 24) -> list:
    """Fetch candle data for volatility calculation via `okx market candles`.
    Returns list of close prices."""
    if not inst_id:
        return []
    data = okx_cli(["market", "candles", inst_id, "--bar", bar, "--limit", str(limit)])
    if not data or not isinstance(data, list):
        return []
    # CLI returns same format: [[ts, open, high, low, close, vol, ...], ...]
    closes = []
    for c in data:
        if isinstance(c, list) and len(c) >= 5:
            closes.append(safe_float(c[4]))
    return closes


# ═══════════════════════════════════════════════════════════════════
# OHLCV CANDLES + TA INDICATORS
# ═══════════════════════════════════════════════════════════════════

def okx_candle_ohlcv(inst_id: str, bar: str = "1H", limit: int = 300) -> list:
    """Fetch OHLCV candles via `okx market candles`. Returns list of
    {time, open, high, low, close, volume} sorted oldest-first."""
    if not inst_id:
        return []
    data = okx_cli(["market", "candles", inst_id, "--bar", bar, "--limit", str(limit)], timeout=20)
    if not data or not isinstance(data, list):
        return []
    candles = []
    for c in data:
        if isinstance(c, list) and len(c) >= 6:
            candles.append({
                "time": int(safe_float(c[0])) // 1000,  # ms → seconds
                "open": safe_float(c[1]),
                "high": safe_float(c[2]),
                "low": safe_float(c[3]),
                "close": safe_float(c[4]),
                "volume": safe_float(c[5]),
            })
    candles.reverse()  # OKX returns newest-first; we want oldest-first
    return candles


def _ema(values: list, period: int) -> list:
    """Exponential moving average. Returns list same length as input."""
    if not values:
        return []
    result = [values[0]]
    k = 2.0 / (period + 1)
    for i in range(1, len(values)):
        result.append(values[i] * k + result[-1] * (1 - k))
    return result


def _rsi_series(closes: list, period: int = 14) -> list:
    """Full RSI series using Wilder's smoothing. Returns list same length as closes."""
    if len(closes) < 2:
        return [50.0] * len(closes)
    deltas = [closes[i] - closes[i - 1] for i in range(1, len(closes))]
    gains = [max(d, 0) for d in deltas]
    losses = [max(-d, 0) for d in deltas]

    result = [50.0]  # pad first value
    if len(gains) < period:
        return [50.0] * len(closes)

    # Seed: simple average of first `period` values
    avg_gain = sum(gains[:period]) / period
    avg_loss = sum(losses[:period]) / period
    # Pad initial period with 50
    result.extend([50.0] * (period - 1))
    # First RSI
    rs = avg_gain / avg_loss if avg_loss > 0 else 100.0
    rsi_val = 100.0 - (100.0 / (1.0 + rs)) if avg_loss > 0 else 100.0
    result.append(rsi_val)

    # Wilder's smoothing for remaining
    for i in range(period, len(gains)):
        avg_gain = (avg_gain * (period - 1) + gains[i]) / period
        avg_loss = (avg_loss * (period - 1) + losses[i]) / period
        if avg_loss == 0:
            result.append(100.0)
        else:
            rs = avg_gain / avg_loss
            result.append(100.0 - (100.0 / (1.0 + rs)))
    return result


def _bb_series(closes: list, period: int = 20, num_std: float = 2.0) -> dict:
    """Bollinger Bands full series. Returns {upper:[], middle:[], lower:[]}."""
    n = len(closes)
    upper, middle, lower = [], [], []
    for i in range(n):
        if i < period - 1:
            upper.append(closes[i])
            middle.append(closes[i])
            lower.append(closes[i])
        else:
            window = closes[i - period + 1: i + 1]
            mid = sum(window) / period
            variance = sum((x - mid) ** 2 for x in window) / period
            std = variance ** 0.5
            upper.append(mid + num_std * std)
            middle.append(mid)
            lower.append(mid - num_std * std)
    return {"upper": upper, "middle": middle, "lower": lower}


def _macd_series(closes: list, fast: int = 12, slow: int = 26, signal: int = 9) -> dict:
    """MACD full series. Returns {macd:[], signal:[], histogram:[]}."""
    ema_fast = _ema(closes, fast)
    ema_slow = _ema(closes, slow)
    macd_line = [f - s for f, s in zip(ema_fast, ema_slow)]
    signal_line = _ema(macd_line, signal)
    histogram = [m - s for m, s in zip(macd_line, signal_line)]
    return {"macd": macd_line, "signal": signal_line, "histogram": histogram}


def compute_ta_indicators(candles: list, rsi_period: int = 14,
                          bb_period: int = 20, bb_std: float = 2.0,
                          macd_fast: int = 12, macd_slow: int = 26,
                          macd_signal: int = 9) -> dict:
    """Compute TA indicators from candles. Returns chart-ready dicts with {time,value} pairs."""
    if not candles:
        return {}
    closes = [c["close"] for c in candles]
    times = [c["time"] for c in candles]

    rsi_vals = _rsi_series(closes, rsi_period)
    bb = _bb_series(closes, bb_period, bb_std)
    macd = _macd_series(closes, macd_fast, macd_slow, macd_signal)

    def to_tv(values):
        return [{"time": t, "value": round(v, 6)} for t, v in zip(times, values)]

    return {
        "rsi": to_tv(rsi_vals),
        "bb": {
            "upper": to_tv(bb["upper"]),
            "middle": to_tv(bb["middle"]),
            "lower": to_tv(bb["lower"]),
        },
        "macd": {
            "macd": to_tv(macd["macd"]),
            "signal": to_tv(macd["signal"]),
            "histogram": to_tv(macd["histogram"]),
        },
    }


def compute_realized_volatility(closes: list) -> dict:
    """Compute realized volatility from hourly close prices.

    Returns annualized vol (sqrt(8760) for hourly data).
    """
    if len(closes) < 3:
        return {"status": "unavailable"}

    # Log returns
    returns = []
    for i in range(1, len(closes)):
        if closes[i - 1] > 0 and closes[i] > 0:
            returns.append(math.log(closes[i] / closes[i - 1]))

    if len(returns) < 2:
        return {"status": "unavailable"}

    mean = sum(returns) / len(returns)
    variance = sum((r - mean) ** 2 for r in returns) / (len(returns) - 1)
    hourly_vol = math.sqrt(variance)
    annualized_vol = hourly_vol * math.sqrt(8760)  # 8760 hours/year

    return {
        "status": "available",
        "realized_vol_1h": round(hourly_vol * 100, 4),
        "realized_vol_annualized_pct": round(annualized_vol * 100, 2),
        "sample_hours": len(returns),
        "source": "okx",
    }


def okx_long_short(inst_id: str) -> dict:
    """Long/short ratio via `okx market indicator top-long-short`."""
    if not inst_id:
        return {"status": "unavailable"}
    # The indicator uses spot instId (BTC-USDT not BTC-USDT-SWAP)
    spot_id = "-".join(inst_id.split("-")[:2])  # BTC-USDT-SWAP → BTC-USDT
    data = okx_cli(["market", "indicator", "top-long-short", spot_id])
    if not data or not isinstance(data, list) or not data:
        return {"status": "unavailable"}

    # Parse nested indicator response
    try:
        entry = data[0].get("data", [{}])[0]
        tf = entry.get("timeframes", {})
        # Get the first available timeframe
        for _, tf_data in tf.items():
            indicators = tf_data.get("indicators", {})
            ls_data = indicators.get("TOPLONGSHORT", [{}])
            if ls_data:
                vals = ls_data[0].get("values", {})
                return {
                    "status": "available",
                    "long_ratio": safe_float(vals.get("longRatio")),
                    "short_ratio": safe_float(vals.get("shortRatio")),
                    "long_short_ratio": safe_float(vals.get("longShortRatio")),
                    "source": "okx-cli",
                }
    except (IndexError, KeyError, TypeError):
        pass
    return {"status": "unavailable"}


def okx_options_summary(inst_family: str) -> dict:
    """Options summary: put/call ratio, max pain from OKX."""
    if not inst_family:
        return {"status": "unavailable", "reason": "No options market for this token"}
    items = okx_data(fetch(f"{OKX_BASE}/public/opt-summary?instFamily={inst_family}"))
    if not items:
        return {"status": "unavailable"}
    r = items[0]
    return {
        "status": "available",
        "put_call_ratio": safe_float(r.get("putCallRatio")) or None,
        "max_pain": safe_float(r.get("maxPain")) or None,
        "call_volume": safe_float(r.get("callVol")) or None,
        "put_volume": safe_float(r.get("putVol")) or None,
        "call_oi": safe_float(r.get("callOi")) or None,
        "put_oi": safe_float(r.get("putOi")) or None,
        "source": "okx",
    }


def okx_taker_volume(ccy: str) -> dict:
    """Taker buy/sell volume ratio from OKX contracts.

    Endpoint: rubik/stat/taker-volume?ccy=BTC&instType=CONTRACTS
    Returns arrays: [ts, sellVol, buyVol] — note: sell first, buy second.
    """
    if not ccy:
        return {"status": "unavailable"}
    url = f"{OKX_BASE}/rubik/stat/taker-volume?ccy={ccy}&instType=CONTRACTS&period=1H"
    items = okx_data(fetch(url))
    if not items:
        return {"status": "unavailable"}
    r = items[0]
    if isinstance(r, list) and len(r) >= 3:
        sell_vol = safe_float(r[1])  # index 1 = sellVol
        buy_vol = safe_float(r[2])   # index 2 = buyVol
    elif isinstance(r, dict):
        buy_vol = safe_float(r.get("buyVol"))
        sell_vol = safe_float(r.get("sellVol"))
    else:
        return {"status": "unavailable"}

    ratio = (buy_vol / sell_vol) if sell_vol > 0 else None
    return {
        "status": "available",
        "buy_volume": round(buy_vol, 2),
        "sell_volume": round(sell_vol, 2),
        "buy_sell_ratio": round(ratio, 4) if ratio else None,
        "interpretation": (
            "aggressive buying" if ratio and ratio > 1.1
            else "aggressive selling" if ratio and ratio < 0.9
            else "balanced"
        ),
        "source": "okx",
    }


def okx_futures_basis(swap_id: str, spot_id: str) -> dict:
    """Futures basis (premium) via `okx market ticker` for swap vs spot."""
    swap_data = okx_cli(["market", "ticker", swap_id])
    spot_data = okx_cli(["market", "ticker", spot_id])
    if not swap_data or not spot_data:
        return {"status": "unavailable"}
    swap_price = safe_float(swap_data[0].get("last")) if isinstance(swap_data, list) and swap_data else 0
    spot_price = safe_float(spot_data[0].get("last")) if isinstance(spot_data, list) and spot_data else 0
    if spot_price == 0:
        return {"status": "unavailable"}
    basis_pct = ((swap_price - spot_price) / spot_price) * 100
    return {
        "status": "available",
        "swap_price": swap_price,
        "spot_price": spot_price,
        "basis_pct": round(basis_pct, 4),
        "interpretation": (
            "contango (longs paying premium)" if basis_pct > 0.01
            else "backwardation (shorts paying premium)" if basis_pct < -0.01
            else "near parity"
        ),
        "source": "okx-cli",
    }


# ═══════════════════════════════════════════════════════════════════
# ON-CHAIN: MVRV + REALIZED PRICE (CoinMetrics free API)
# ═══════════════════════════════════════════════════════════════════

def coinmetrics_mvrv(asset: str, spot_price: float = 0) -> dict:
    """Fetch MVRV ratio from CoinMetrics community API (free, no key).

    MVRV = Market Cap / Realized Cap.
    >3.5 = historically overheated. <1.0 = undervalued (holders underwater).
    """
    cm_asset = {"BTC": "btc", "ETH": "eth"}.get(asset.upper(), "")
    if not cm_asset:
        return {"status": "unavailable", "reason": "MVRV only available for BTC/ETH"}

    end = datetime.now(timezone.utc)
    start_str = (end - __import__("datetime").timedelta(days=30)).strftime("%Y-%m-%dT00:00:00Z")

    url = (
        f"https://community-api.coinmetrics.io/v4/timeseries/asset-metrics"
        f"?assets={cm_asset}&metrics=CapMVRVCur&frequency=1d"
        f"&start_time={start_str}&page_size=60"
    )
    data = fetch(url, timeout=15)
    if is_error(data):
        return {"status": "unavailable"}

    items = data.get("data", [])
    if not items:
        return {"status": "unavailable"}

    # Filter to this asset
    values = [safe_float(i.get("CapMVRVCur")) for i in items if i.get("asset") == cm_asset and i.get("CapMVRVCur")]
    if not values:
        return {"status": "unavailable"}

    latest = values[-1]
    high_30d = max(values)
    low_30d = min(values)
    avg_30d = sum(values) / len(values)

    # Realized price = spot / MVRV
    realized_price = round(spot_price / latest, 2) if latest > 0 and spot_price > 0 else None

    # Interpretation
    if latest > 3.5:
        zone = "overheated (historically top territory)"
    elif latest > 2.5:
        zone = "elevated (caution)"
    elif latest > 1.5:
        zone = "healthy"
    elif latest > 1.0:
        zone = "undervalued (accumulation zone)"
    else:
        zone = "deep value (holders underwater)"

    return {
        "status": "available",
        "mvrv": round(latest, 4),
        "mvrv_30d_high": round(high_30d, 4),
        "mvrv_30d_low": round(low_30d, 4),
        "mvrv_30d_avg": round(avg_30d, 4),
        "realized_price": realized_price,
        "zone": zone,
        "data_points": len(values),
        "source": "coinmetrics",
    }


# ═══════════════════════════════════════════════════════════════════
# OPTIONS: GAMMA WALL + SKEW (OKX option chain)
# ═══════════════════════════════════════════════════════════════════

def okx_gamma_wall(inst_family: str, spot_price: float = 0) -> dict:
    """Compute gamma exposure by strike from OKX option chain.

    Gamma wall = strike with largest net gamma × OI. Market makers who are
    net short options have negative gamma — price tends to stick near high-gamma
    strikes (pin risk) and accelerate away from low-gamma zones.
    """
    if not inst_family:
        return {"status": "unavailable", "reason": "No options market"}

    # Fetch opt-summary (has Greeks: gammaBS, deltaBS, markVol per instrument)
    summary_data = fetch(f"{OKX_BASE}/public/opt-summary?instFamily={inst_family}", timeout=15)
    summary_items = okx_data(summary_data) if not is_error(summary_data) else []

    # Fetch per-instrument OI
    oi_data = fetch(f"{OKX_BASE}/public/open-interest?instType=OPTION&instFamily={inst_family}", timeout=15)
    oi_items = okx_data(oi_data) if not is_error(oi_data) else []

    if not summary_items or not oi_items:
        return {"status": "unavailable"}

    # Build OI map: instId → oiCcy
    oi_map = {}
    for item in oi_items:
        inst_id = item.get("instId", "")
        oi_map[inst_id] = safe_float(item.get("oiCcy"))

    # Build gamma map: aggregate gamma × OI by strike
    from collections import defaultdict
    gamma_by_strike = defaultdict(float)
    call_oi_by_strike = defaultdict(float)
    put_oi_by_strike = defaultdict(float)

    for t in summary_items:
        inst_id = t.get("instId", "")
        parts = inst_id.split("-")
        if len(parts) != 5:
            continue

        strike = safe_float(parts[3])
        cp = parts[4]  # C or P
        gamma_bs = safe_float(t.get("gammaBS"))
        oi = oi_map.get(inst_id, 0)

        if oi <= 0 or gamma_bs <= 0:
            continue

        # Net gamma exposure (MM perspective: short options → negative gamma)
        # Calls: MM short gamma if calls are bought
        # Puts: MM short gamma if puts are bought
        # For gamma wall: just aggregate |gamma × OI| per strike
        gamma_exposure = gamma_bs * oi * spot_price * spot_price * 0.01 if spot_price > 0 else gamma_bs * oi
        gamma_by_strike[strike] += gamma_exposure

        if cp == "C":
            call_oi_by_strike[strike] += oi
        else:
            put_oi_by_strike[strike] += oi

    if not gamma_by_strike:
        return {"status": "unavailable"}

    # Sort by gamma exposure, find top 5 gamma walls
    sorted_strikes = sorted(gamma_by_strike.items(), key=lambda x: -x[1])
    top_5 = sorted_strikes[:5]

    # Find the biggest gamma wall
    wall_strike = top_5[0][0]
    wall_gamma = top_5[0][1]

    # Classify: above spot = resistance, below spot = support
    if spot_price > 0:
        wall_type = "resistance (price magnet above)" if wall_strike > spot_price else "support (price magnet below)"
    else:
        wall_type = "unknown"

    return {
        "status": "available",
        "gamma_wall_strike": wall_strike,
        "gamma_wall_exposure": round(wall_gamma, 2),
        "wall_type": wall_type,
        "top_strikes": [{"strike": s, "gamma": round(g, 2)} for s, g in top_5],
        "call_oi_strikes": {str(int(k)): round(v, 4) for k, v in sorted(call_oi_by_strike.items()) if v > 0.1},
        "put_oi_strikes": {str(int(k)): round(v, 4) for k, v in sorted(put_oi_by_strike.items()) if v > 0.1},
        "total_strikes_with_oi": len(gamma_by_strike),
        "source": "okx",
    }


def okx_skew(inst_family: str) -> dict:
    """Compute 25-delta risk reversal (skew) from OKX option chain.

    Skew = 25d Put IV - 25d Call IV.
    Positive skew → puts are more expensive → market is paying for downside protection (bearish).
    Negative skew → calls are more expensive → market is bidding for upside (bullish).

    Uses the nearest weekly expiry for most liquid data.
    """
    if not inst_family:
        return {"status": "unavailable", "reason": "No options market"}

    # Use opt-summary which has full Greeks (deltaBS, markVol, bidVol, askVol)
    summary_data = fetch(f"{OKX_BASE}/public/opt-summary?instFamily={inst_family}", timeout=15)
    summary_items = okx_data(summary_data) if not is_error(summary_data) else []

    if not summary_items:
        return {"status": "unavailable"}

    # Group by expiry
    from collections import defaultdict
    by_expiry = defaultdict(list)
    for t in summary_items:
        inst_id = t.get("instId", "")
        parts = inst_id.split("-")
        if len(parts) != 5:
            continue
        exp = parts[2]
        strike = safe_float(parts[3])
        cp = parts[4]
        delta = safe_float(t.get("deltaBS"))  # Black-Scholes delta
        mark_vol = safe_float(t.get("markVol"))  # Mark IV
        ask_vol = safe_float(t.get("askVol"))
        bid_vol = safe_float(t.get("bidVol"))
        mid_vol = (ask_vol + bid_vol) / 2 if ask_vol > 0 and bid_vol > 0 else mark_vol

        if mid_vol <= 0:
            continue

        by_expiry[exp].append({
            "strike": strike, "cp": cp, "delta": delta,
            "iv": mid_vol, "mark_vol": mark_vol,
        })

    if not by_expiry:
        return {"status": "unavailable"}

    # Use nearest expiry with enough data
    sorted_expiries = sorted(by_expiry.keys())
    target_exp = None
    for exp in sorted_expiries:
        if len(by_expiry[exp]) >= 6:  # need at least a few strikes
            target_exp = exp
            break

    if not target_exp:
        return {"status": "unavailable"}

    options = by_expiry[target_exp]

    # Find 25-delta put and 25-delta call
    # 25d call: delta ≈ +0.25, 25d put: delta ≈ -0.25
    calls = [o for o in options if o["cp"] == "C"]
    puts = [o for o in options if o["cp"] == "P"]

    # Find closest to 25-delta
    call_25d = min(calls, key=lambda o: abs(abs(o["delta"]) - 0.25), default=None) if calls else None
    put_25d = min(puts, key=lambda o: abs(abs(o["delta"]) - 0.25), default=None) if puts else None

    # Find ATM (closest to 50-delta)
    atm_call = min(calls, key=lambda o: abs(abs(o["delta"]) - 0.5), default=None) if calls else None

    if not call_25d or not put_25d:
        return {"status": "unavailable"}

    skew_25d = put_25d["iv"] - call_25d["iv"]  # positive = bearish
    atm_iv = atm_call["iv"] if atm_call else (call_25d["iv"] + put_25d["iv"]) / 2

    # Butterfly: (25d put IV + 25d call IV) / 2 - ATM IV
    # Measures tail risk pricing
    butterfly = ((put_25d["iv"] + call_25d["iv"]) / 2) - atm_iv if atm_call else None

    # Interpretation
    if skew_25d > 0.05:
        skew_signal = "heavily bearish — puts very expensive"
    elif skew_25d > 0.02:
        skew_signal = "bearish — downside protection bid"
    elif skew_25d > -0.02:
        skew_signal = "neutral"
    elif skew_25d > -0.05:
        skew_signal = "bullish — calls bid over puts"
    else:
        skew_signal = "heavily bullish — call skew dominant"

    return {
        "status": "available",
        "expiry": target_exp,
        "skew_25d": round(skew_25d * 100, 2),  # as percentage points
        "put_25d_iv": round(put_25d["iv"] * 100, 2),
        "call_25d_iv": round(call_25d["iv"] * 100, 2),
        "put_25d_strike": put_25d["strike"],
        "call_25d_strike": call_25d["strike"],
        "atm_iv": round(atm_iv * 100, 2),
        "butterfly": round(butterfly * 100, 2) if butterfly is not None else None,
        "signal": skew_signal,
        "source": "okx",
    }


def okx_liquidation_proxy(inst_id: str) -> dict:
    """Estimate liquidation pressure from L/S ratio movement.

    Uses the long/short ratio. Sharp swings suggest forced closures.
    """
    ls = okx_long_short(inst_id)
    if ls.get("status") != "available":
        return {"status": "unavailable"}

    long_r = safe_float(ls.get("long_ratio"))
    short_r = safe_float(ls.get("short_ratio"))
    ratio = safe_float(ls.get("long_short_ratio"))

    # Simple heuristic from current L/S snapshot
    if long_r == 0 and short_r == 0:
        return {"status": "unavailable"}

    long_pct = round(long_r * 100, 1) if long_r else 50.0

    if long_pct > 55:
        pressure = "moderate — crowded long, liquidation risk above"
        bias = "shorts under pressure"
    elif long_pct < 45:
        pressure = "moderate — crowded short, squeeze risk"
        bias = "longs under pressure"
    else:
        pressure = "low — orderly market"
        bias = "balanced"

    return {
        "status": "available",
        "latest_ls_ratio": round(ratio, 4) if ratio else None,
        "pressure": pressure,
        "bias": bias,
        "long_pct": long_pct,
        "source": "okx-cli",
    }


# ═══════════════════════════════════════════════════════════════════
# MACRO / SENTIMENT (token-independent)
# ═══════════════════════════════════════════════════════════════════

def fear_greed() -> dict:
    """Fear & Greed Index."""
    data = fetch("https://api.alternative.me/fng/?limit=7")
    if is_error(data):
        return {"status": "unavailable"}
    items = data.get("data", [])
    if not items:
        return {"status": "unavailable"}
    current = items[0]
    value = int(current.get("value", 0))
    classification = current.get("value_classification", "Unknown")

    # 7-day trend
    values = [int(i.get("value", 0)) for i in items]
    if len(values) >= 2:
        recent = sum(values[:3]) / min(3, len(values))
        older = sum(values[3:]) / max(len(values) - 3, 1)
        trend = "improving" if recent > older + 3 else "deteriorating" if recent < older - 3 else "flat"
    else:
        trend = "unknown"

    return {
        "status": "available",
        "value": value,
        "classification": classification,
        "trend_7d": trend,
        "history_7d": [{"value": int(i.get("value", 0)), "label": i.get("value_classification", ""),
                        "date": i.get("timestamp", "")} for i in items],
        "source": "alternative.me",
    }


def coingecko_global() -> dict:
    """Global market data from CoinGecko."""
    data = fetch("https://api.coingecko.com/api/v3/global", timeout=15)
    if is_error(data):
        return {"status": "unavailable"}
    gd = data.get("data", {})
    if not gd:
        return {"status": "unavailable"}
    return {
        "status": "available",
        "btc_dominance": round(gd.get("market_cap_percentage", {}).get("btc", 0), 2),
        "eth_dominance": round(gd.get("market_cap_percentage", {}).get("eth", 0), 2),
        "total_market_cap_usd": gd.get("total_market_cap", {}).get("usd", 0),
        "total_volume_24h_usd": gd.get("total_volume", {}).get("usd", 0),
        "market_cap_change_24h_pct": round(gd.get("market_cap_change_percentage_24h_usd", 0), 2),
        "source": "coingecko",
    }


def stablecoin_data() -> dict:
    """Stablecoin market cap from DefiLlama."""
    data = fetch("https://stablecoins.llama.fi/stablecoins?includePrices=true", timeout=15)
    if is_error(data):
        return {"status": "unavailable"}
    coins = data.get("peggedAssets", [])
    total_mcap = 0
    top_stables = []
    for c in coins[:5]:
        chains = c.get("chainCirculating", {})
        mcap = sum(v.get("current", {}).get("peggedUSD", 0) for v in chains.values()) if chains else 0
        if mcap == 0:
            mcap = c.get("circulating", {}).get("peggedUSD", 0)
        total_mcap += mcap
        top_stables.append({"name": c.get("name", ""), "symbol": c.get("symbol", ""), "mcap": round(mcap)})
    return {
        "status": "available",
        "total_stablecoin_mcap": round(total_mcap),
        "top_stablecoins": top_stables,
        "source": "defillama",
    }


# ═══════════════════════════════════════════════════════════════════
# ONCHAINOS — ON-CHAIN DEX / SMART MONEY DATA
# ═══════════════════════════════════════════════════════════════════

ONCHAINOS_CLI = os.environ.get("ONCHAINOS_CLI_PATH", os.path.expanduser("~/.local/bin/onchainos"))


def _onchainos(args: list[str], timeout: int = 15) -> dict | list | None:
    """Run onchainos CLI and return parsed JSON output.
    OnchainOS outputs JSON by default — no extra flags needed."""
    cmd = [ONCHAINOS_CLI] + args
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        if result.returncode != 0:
            return None
        return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError, FileNotFoundError):
        return None


def onchain_smart_money_signals(chains: list[str] = None) -> dict:
    """Aggregate smart money + whale buy/sell signals across chains.

    Uses onchainos signal list for each chain, aggregates into
    net buy/sell pressure and top movers.
    """
    if chains is None:
        chains = ["ethereum", "solana", "base"]

    all_signals = []
    for chain in chains:
        data = _onchainos([
            "signal", "list", "--chain", chain,
            "--wallet-type", "1,3",  # smart money + whales
            "--min-amount-usd", "10000",
        ])
        if data and isinstance(data, dict):
            items = data.get("data", [])
            for s in items:
                tok = s.get("token", {})
                sold_pct = safe_float(s.get("soldRatioPercent"))
                amount = safe_float(s.get("amountUsd"))
                all_signals.append({
                    "chain": chain,
                    "symbol": tok.get("symbol", "?"),
                    "amount_usd": amount,
                    "action": "sell" if sold_pct > 50 else "buy",
                    "wallet_type": "smart_money" if s.get("walletType") == "1" else "whale",
                    "wallet_count": int(s.get("triggerWalletCount", 0)),
                    "mcap": safe_float(tok.get("marketCapUsd")),
                })

    if not all_signals:
        return {"status": "unavailable"}

    # Aggregate net flow
    buy_vol = sum(s["amount_usd"] for s in all_signals if s["action"] == "buy")
    sell_vol = sum(s["amount_usd"] for s in all_signals if s["action"] == "sell")
    net_flow = buy_vol - sell_vol
    buy_count = sum(1 for s in all_signals if s["action"] == "buy")
    sell_count = sum(1 for s in all_signals if s["action"] == "sell")

    # Top 5 by amount
    top_signals = sorted(all_signals, key=lambda x: -x["amount_usd"])[:5]

    if buy_vol + sell_vol > 0:
        buy_pct = round(buy_vol / (buy_vol + sell_vol) * 100, 1)
    else:
        buy_pct = 50.0

    return {
        "status": "available",
        "total_signals": len(all_signals),
        "buy_count": buy_count,
        "sell_count": sell_count,
        "buy_volume_usd": round(buy_vol, 2),
        "sell_volume_usd": round(sell_vol, 2),
        "net_flow_usd": round(net_flow, 2),
        "buy_pct": buy_pct,
        "sentiment": (
            "strong buying" if buy_pct > 65
            else "buying" if buy_pct > 55
            else "strong selling" if buy_pct < 35
            else "selling" if buy_pct < 45
            else "balanced"
        ),
        "top_signals": [{
            "chain": s["chain"],
            "symbol": s["symbol"],
            "action": s["action"],
            "amount_usd": round(s["amount_usd"], 0),
            "wallet_type": s["wallet_type"],
            "wallets": s["wallet_count"],
        } for s in top_signals],
        "chains_scanned": chains,
        "source": "onchainos",
    }


def onchain_hot_tokens() -> dict:
    """Get trending tokens across chains from OnchainOS.

    Returns top tokens by DEX volume (24h) with mcap > $10M.
    Shows what's hot on-chain vs what's hot on CEX.
    """
    data = _onchainos([
        "token", "hot-tokens",
        "--rank-by", "5",        # sort by volume
        "--time-frame", "4",     # 24h
        "--market-cap-min", "10000000",  # >$10M mcap
    ])
    if not data or not isinstance(data, dict):
        return {"status": "unavailable"}

    items = data.get("data", [])
    if not items:
        return {"status": "unavailable"}

    chain_map = {"1": "ETH", "56": "BSC", "501": "SOL", "8453": "BASE", "42161": "ARB", "137": "MATIC"}
    tokens = []
    for t in items[:12]:
        chain_id = t.get("chainIndex", "")
        tokens.append({
            "symbol": t.get("tokenSymbol", "?"),
            "chain": chain_map.get(chain_id, f"chain:{chain_id}"),
            "price": safe_float(t.get("price")),
            "change_24h_pct": safe_float(t.get("change")),
            "volume_24h": safe_float(t.get("volume")),
            "mcap": safe_float(t.get("marketCap")),
            "liquidity": safe_float(t.get("liquidity")),
            "txs_24h": int(t.get("txs", 0)),
            "unique_traders": int(t.get("uniqueTraders", 0)),
            "net_inflow_usd": safe_float(t.get("inflowUsd")),
        })

    # Aggregate stats
    total_vol = sum(t["volume_24h"] for t in tokens)
    net_inflow = sum(t["net_inflow_usd"] for t in tokens)
    chains_active = list(set(t["chain"] for t in tokens))

    return {
        "status": "available",
        "top_tokens": tokens,
        "total_dex_volume_24h": round(total_vol, 0),
        "net_inflow_usd": round(net_inflow, 0),
        "chains_active": sorted(chains_active),
        "token_count": len(tokens),
        "source": "onchainos",
    }


# ═══════════════════════════════════════════════════════════════════
# PER-TOKEN AGGREGATOR (OKX CeFi CLI primary)
# ═══════════════════════════════════════════════════════════════════

def analyze_token(symbol: str) -> dict:
    """Fetch all available indicators for a token via OKX CeFi CLI.

    Uses concurrent CLI calls for speed.
    """
    token = TOKEN_MAP.get(symbol.upper())
    if not token:
        return {"symbol": symbol, "error": f"Unknown token. Supported: {', '.join(sorted(TOKEN_MAP.keys()))}"}

    result = {
        "symbol": symbol.upper(),
        "tier": token["tier"],
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "derivatives": {},
        "market_structure": {},
    }

    swap_id = token["okx_swap"]
    spot_id = token["okx_spot"]
    ccy = symbol.upper()

    # ── Parallel CLI calls for speed ──
    futures = {}
    with ThreadPoolExecutor(max_workers=8) as pool:
        futures["ticker"] = pool.submit(okx_ticker, spot_id)
        futures["candles_vol"] = pool.submit(okx_candle_history, spot_id, "1H", 24)
        futures["funding"] = pool.submit(okx_funding, swap_id)
        futures["funding_hist"] = pool.submit(okx_funding_history, swap_id, 6)
        futures["oi"] = pool.submit(okx_open_interest, swap_id)
        futures["oi_hist"] = pool.submit(okx_oi_history, swap_id)
        futures["long_short"] = pool.submit(okx_long_short, swap_id)
        futures["taker"] = pool.submit(okx_taker_volume, ccy)
        futures["basis"] = pool.submit(okx_futures_basis, swap_id, spot_id)
        futures["options"] = pool.submit(okx_options_summary, token["okx_family"])

    # ── Collect results ──
    result["market_structure"]["ticker_24h"] = futures["ticker"].result()
    closes = futures["candles_vol"].result()
    result["market_structure"]["realized_vol"] = compute_realized_volatility(closes)
    result["derivatives"]["funding"] = futures["funding"].result()
    result["derivatives"]["funding_history"] = futures["funding_hist"].result()
    result["derivatives"]["open_interest"] = futures["oi"].result()
    result["derivatives"]["oi_history"] = futures["oi_hist"].result()
    result["market_structure"]["long_short"] = futures["long_short"].result()
    result["market_structure"]["taker_volume"] = futures["taker"].result()
    result["derivatives"]["basis"] = futures["basis"].result()
    result["derivatives"]["options"] = futures["options"].result()

    # ── Liquidation proxy (from L/S data) ──
    result["market_structure"]["liquidations"] = okx_liquidation_proxy(swap_id)

    # ── MVRV + Realized Price (BTC/ETH only — CoinMetrics) ──
    spot_price = (result["market_structure"].get("ticker_24h") or {}).get("price", 0)
    if ccy in ("BTC", "ETH"):
        result["on_chain"] = {"mvrv": coinmetrics_mvrv(ccy, spot_price)}

    # ── Gamma Wall + Skew (direct HTTP — complex option chain computation) ──
    if token["okx_family"]:
        result["derivatives"]["gamma_wall"] = okx_gamma_wall(token["okx_family"], spot_price)
        result["derivatives"]["skew"] = okx_skew(token["okx_family"])

    return result


# ═══════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════

def main():
    tokens = sys.argv[1:] if len(sys.argv) > 1 else ["BTC"]
    if tokens == ["--all"]:
        tokens = sorted(TOKEN_MAP.keys())

    output = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "version": "3.0",
        "data_priority": "OKX CeFi CLI primary, HTTP for options chain + external macro",
        "tokens": {},
        "macro": {},
    }

    # Macro (token-independent)
    print("Fetching macro data...", file=sys.stderr)
    output["macro"]["fear_greed"] = fear_greed()
    output["macro"]["global"] = coingecko_global()
    output["macro"]["stablecoins"] = stablecoin_data()

    # Per-token
    for t in tokens:
        t = t.upper()
        print(f"Fetching {t} (OKX CeFi CLI)...", file=sys.stderr)
        output["tokens"][t] = analyze_token(t)
        time.sleep(0.1)

    # Summary
    available = 0
    unavailable = 0
    for t_data in output["tokens"].values():
        for section in ["derivatives", "market_structure", "on_chain"]:
            for k, v in t_data.get(section, {}).items():
                if isinstance(v, dict):
                    if v.get("status") == "available":
                        available += 1
                    elif v.get("status") == "unavailable":
                        unavailable += 1
    for k, v in output["macro"].items():
        if isinstance(v, dict):
            if v.get("status") == "available":
                available += 1
            elif v.get("status") == "unavailable":
                unavailable += 1

    output["_summary"] = {
        "indicators_available": available,
        "indicators_unavailable": unavailable,
        "coverage_pct": round(available / max(available + unavailable, 1) * 100, 1),
    }

    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
