"""
Filter evaluators — 10 market-condition + 13 token-safety = 23 primitives.

Each function: evaluate_<type>(ctx, params) -> bool
  True = filter passes (entry allowed), False = filter blocks entry.
"""
from __future__ import annotations

from typing import Any
from datetime import datetime, timezone

from primitives.entry import MarketContext
from onchainos import SafetyTags


# ── Market-condition filters ─────────────────────────────────────────────────

def evaluate_time_window(ctx: MarketContext, params: dict) -> bool:
    """MF-01: Only trade during specified UTC hours."""
    start = params["start_hour"]
    end = params["end_hour"]
    weekdays_only = params.get("weekdays_only", False)
    now = datetime.now(timezone.utc)
    hour = now.hour
    day = now.weekday()  # 0=Mon, 6=Sun
    if weekdays_only and day >= 5:
        return False
    if start <= end:
        return start <= hour < end
    else:  # wraps midnight
        return hour >= start or hour < end


def evaluate_volatility_range(ctx: MarketContext, params: dict) -> bool:
    """MF-02: Only trade when ATR% is in [min, max]."""
    atr_period = params["atr_period"]
    min_pct = params.get("min_pct", 0)
    max_pct = params.get("max_pct", 999)
    if len(ctx.bars) < atr_period + 1:
        return False
    # Compute ATR
    trs: list[float] = []
    for i in range(-atr_period, 0):
        bar = ctx.bars[i]
        prev = ctx.bars[i - 1]
        tr = max(bar.high - bar.low, abs(bar.high - prev.close), abs(bar.low - prev.close))
        trs.append(tr)
    atr = sum(trs) / len(trs)
    atr_pct = (atr / ctx.current_price * 100) if ctx.current_price > 0 else 0
    return min_pct <= atr_pct <= max_pct


def evaluate_volume_minimum(ctx: MarketContext, params: dict) -> bool:
    """MF-03: Only trade when 24h volume > threshold."""
    min_usd = params["min_usd_24h"]
    # Approximate: sum volume of recent bars covering ~24h
    # Engine should provide a pre-computed 24h volume if available
    if not ctx.bars:
        return False
    # Heuristic: sum all available bar volumes as approximation
    total = sum(b.volume for b in ctx.bars[-288:])  # 288 x 5m = 24h
    return total >= min_usd


def evaluate_cooldown(ctx: MarketContext, params: dict) -> bool:
    """MF-04: Wait N bars after last trade close.
    Engine must track last_trade_bar_idx in ctx and pass it here.
    For now, always passes — engine enforces.
    """
    return True  # Engine tracks cooldown state externally


def evaluate_market_regime(ctx: MarketContext, params: dict) -> bool:
    """MF-05: Only trade in specified regime (by MA slope)."""
    regime = params["regime"]
    if regime == "any":
        return True
    ma_period = params.get("ma_period", 200)
    sma = ctx.sma.get(ma_period, [])
    if len(sma) < 2:
        return False
    slope = sma[-1] - sma[-2]
    if regime == "up":
        return slope > 0
    elif regime == "down":
        return slope < 0
    else:  # range
        return abs(slope) / (sma[-1] if sma[-1] > 0 else 1) < 0.001


def evaluate_price_range(ctx: MarketContext, params: dict) -> bool:
    """MF-06: Only trade when price in [min, max] USD."""
    min_p = params.get("min_price", 0)
    max_p = params.get("max_price", float("inf"))
    return min_p <= ctx.current_price <= max_p


def evaluate_btc_overlay(ctx: MarketContext, params: dict) -> bool:
    """MF-07: Only trade alts when BTC is favorable.
    Engine must provide BTC data in ctx. Stub: always passes.
    """
    # TODO: engine provides BTC MA / candle data
    return True


def evaluate_top_zone_guard(ctx: MarketContext, params: dict) -> bool:
    """MF-08: Don't buy if price is in top X% of recent range."""
    max_zone_pct = params["max_zone_pct"]
    lookback = params["lookback_bars"]
    if len(ctx.bars) < lookback:
        return False
    window = ctx.bars[-lookback:]
    hi = max(b.high for b in window)
    lo = min(b.low for b in window)
    if hi == lo:
        return True
    zone_pct = (ctx.current_price - lo) / (hi - lo) * 100
    return zone_pct <= max_zone_pct


def evaluate_mcap_range(ctx: MarketContext, params: dict) -> bool:
    """MF-09: Only trade when token mcap in range. Live-only (OnchainOS)."""
    min_usd = params.get("min_usd", 0)
    max_usd = params.get("max_usd", float("inf"))
    if ctx.onchainos is None:
        return True  # Can't check without OnchainOS
    tags = ctx.onchainos.get_safety_tags(ctx.token)
    return min_usd <= tags.mcap_usd <= max_usd


def evaluate_launch_age(ctx: MarketContext, params: dict) -> bool:
    """MF-10: Filter by token age since launch. Live-only (OnchainOS)."""
    min_h = params.get("min_hours", 0)
    max_h = params.get("max_hours", float("inf"))
    if ctx.onchainos is None:
        return True
    tags = ctx.onchainos.get_safety_tags(ctx.token)
    return min_h <= tags.launch_age_hours <= max_h


# ── Token-safety filters (all live_only, all OnchainOS-backed) ───────────────

def _get_tags(ctx: MarketContext) -> SafetyTags | None:
    if ctx.onchainos is None:
        return None
    return ctx.onchainos.get_safety_tags(ctx.token)


def evaluate_honeypot_check(ctx: MarketContext, params: dict) -> bool:
    """TF-01: Reject if honeypot."""
    tags = _get_tags(ctx)
    if tags is None:
        return False  # Fail-safe: block if we can't check
    return not tags.honeypot


def evaluate_lp_locked(ctx: MarketContext, params: dict) -> bool:
    """TF-02: LP must be locked above threshold."""
    min_pct = params["min_pct_locked"]
    min_days = params.get("min_lock_days", 0)
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.lp_locked_pct >= min_pct and tags.lp_lock_days >= min_days


def evaluate_buy_tax_max(ctx: MarketContext, params: dict) -> bool:
    """TF-03: Reject if buy tax exceeds threshold."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.buy_tax_pct <= params["max_pct"]


def evaluate_sell_tax_max(ctx: MarketContext, params: dict) -> bool:
    """TF-04: Reject if sell tax exceeds threshold."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.sell_tax_pct <= params["max_pct"]


def evaluate_liquidity_min(ctx: MarketContext, params: dict) -> bool:
    """TF-05: Minimum pool liquidity."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.liquidity_usd >= params["min_usd"]


def evaluate_top_holders_max(ctx: MarketContext, params: dict) -> bool:
    """TF-06: Reject if top-N holders too concentrated."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.top_holders_pct <= params["max_pct"]


def evaluate_bundler_ratio_max(ctx: MarketContext, params: dict) -> bool:
    """TF-07: Reject if bundler ratio too high."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.bundler_ratio_pct <= params["max_pct"]


def evaluate_dev_holding_max(ctx: MarketContext, params: dict) -> bool:
    """TF-08: Reject if dev holds too much."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.dev_holding_pct <= params["max_pct"]


def evaluate_insider_holding_max(ctx: MarketContext, params: dict) -> bool:
    """TF-09: Reject if insiders hold too much."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.insider_holding_pct <= params["max_pct"]


def evaluate_fresh_wallet_ratio_max(ctx: MarketContext, params: dict) -> bool:
    """TF-10: Reject if too many fresh wallets."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.fresh_wallet_ratio_pct <= params["max_pct"]


def evaluate_smart_money_present_min(ctx: MarketContext, params: dict) -> bool:
    """TF-11: Require >= N smart-money wallets currently holding. State check (G-05)."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.smart_money_count >= params["min_wallets"]


def evaluate_phishing_exclude(ctx: MarketContext, params: dict) -> bool:
    """TF-12: Reject if on phishing blacklist."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return not tags.phishing_flagged


def evaluate_whale_concentration_max(ctx: MarketContext, params: dict) -> bool:
    """TF-13: Reject if single whale too large."""
    tags = _get_tags(ctx)
    if tags is None:
        return False
    return tags.whale_max_pct <= params["max_pct"]


# ── Dispatcher ───────────────────────────────────────────────────────────────

FILTER_EVALUATORS: dict[str, Any] = {
    "time_window": evaluate_time_window,
    "volatility_range": evaluate_volatility_range,
    "volume_minimum": evaluate_volume_minimum,
    "cooldown": evaluate_cooldown,
    "market_regime": evaluate_market_regime,
    "price_range": evaluate_price_range,
    "btc_overlay": evaluate_btc_overlay,
    "top_zone_guard": evaluate_top_zone_guard,
    "mcap_range": evaluate_mcap_range,
    "launch_age": evaluate_launch_age,
    "honeypot_check": evaluate_honeypot_check,
    "lp_locked": evaluate_lp_locked,
    "buy_tax_max": evaluate_buy_tax_max,
    "sell_tax_max": evaluate_sell_tax_max,
    "liquidity_min": evaluate_liquidity_min,
    "top_holders_max": evaluate_top_holders_max,
    "bundler_ratio_max": evaluate_bundler_ratio_max,
    "dev_holding_max": evaluate_dev_holding_max,
    "insider_holding_max": evaluate_insider_holding_max,
    "fresh_wallet_ratio_max": evaluate_fresh_wallet_ratio_max,
    "smart_money_present_min": evaluate_smart_money_present_min,
    "phishing_exclude": evaluate_phishing_exclude,
    "whale_concentration_max": evaluate_whale_concentration_max,
}


def evaluate_filters(ctx: MarketContext, filters: list[dict]) -> tuple[bool, list[str]]:
    """
    Evaluate all filters. ALL must pass for entry to be allowed.
    Returns (all_pass, list_of_failing_filter_types).
    """
    failures: list[str] = []
    for filt in filters:
        ftype = filt.get("type", "")
        evaluator = FILTER_EVALUATORS.get(ftype)
        if evaluator is None:
            failures.append(f"unknown:{ftype}")
            continue
        params = {k: v for k, v in filt.items() if k != "type"}
        if not evaluator(ctx, params):
            failures.append(ftype)
    return len(failures) == 0, failures
