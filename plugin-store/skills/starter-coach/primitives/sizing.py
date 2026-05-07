"""
Sizing evaluators — 3 primitives.

Each function: compute_size(equity_usd, ctx, params) -> float (USD to trade)
L3 hard bound: max 10% of equity per trade.
"""
from __future__ import annotations

from typing import Any
from primitives.entry import MarketContext

L3_MAX_PCT = 10.0  # hard bound


def compute_fixed_pct(equity_usd: float, ctx: MarketContext, params: dict) -> float:
    """S-01: Fixed % of current equity."""
    pct = min(params["pct"], L3_MAX_PCT)
    return equity_usd * pct / 100


def compute_fixed_usd(equity_usd: float, ctx: MarketContext, params: dict) -> float:
    """S-02: Fixed USD amount."""
    usd = params["usd"]
    max_allowed = equity_usd * L3_MAX_PCT / 100
    return min(usd, max_allowed)


def compute_volatility_scaled(equity_usd: float, ctx: MarketContext, params: dict) -> float:
    """S-03: Size inversely scaled by ATR. Smaller in volatile markets."""
    target_risk = params["target_risk_pct"]
    atr_period = params.get("atr_period", 14)
    if len(ctx.bars) < atr_period + 1:
        return 0.0
    # Compute ATR
    trs: list[float] = []
    for i in range(-atr_period, 0):
        bar = ctx.bars[i]
        prev = ctx.bars[i - 1]
        tr = max(bar.high - bar.low, abs(bar.high - prev.close), abs(bar.low - prev.close))
        trs.append(tr)
    atr = sum(trs) / len(trs)
    if atr == 0 or ctx.current_price == 0:
        return 0.0
    atr_pct = atr / ctx.current_price * 100
    # Position size = (target_risk% of equity) / ATR%
    raw_size = (equity_usd * target_risk / 100) / (atr_pct / 100)
    max_allowed = equity_usd * L3_MAX_PCT / 100
    return min(raw_size, max_allowed)


SIZING_EVALUATORS: dict[str, Any] = {
    "fixed_pct": compute_fixed_pct,
    "fixed_usd": compute_fixed_usd,
    "volatility_scaled": compute_volatility_scaled,
}


def compute_size(equity_usd: float, ctx: MarketContext, sizing_spec: dict) -> float:
    """Dispatch to the correct sizing function."""
    stype = sizing_spec["type"]
    params = {k: v for k, v in sizing_spec.items() if k != "type"}
    fn = SIZING_EVALUATORS.get(stype)
    if fn is None:
        raise ValueError(f"Unknown sizing type: {stype}")
    size = fn(equity_usd, ctx, params)
    # Final L3 clamp
    max_allowed = equity_usd * L3_MAX_PCT / 100
    return max(0.0, min(size, max_allowed))
