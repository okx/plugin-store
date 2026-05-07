"""
Grid meta-template expansion.

Expands a grid shorthand into a full strategy spec with composed primitives:
  - time_schedule (entry)
  - price_range (filter)
  - fixed_usd (sizing)
  - per-slot take_profit
  - portfolio stop_loss
  - max_concurrent_positions risk overlay
"""
from __future__ import annotations


def grid_expand(grid_params: dict, meta: dict, instrument: dict) -> dict:
    """
    Expand grid meta-template into a full spec.

    grid_params: {price_min, price_max, levels, usd_per_level,
                  take_profit_per_level_pct?, portfolio_stop_loss_pct?}
    """
    price_min = grid_params["price_min"]
    price_max = grid_params["price_max"]
    levels = grid_params["levels"]
    usd_per_level = grid_params["usd_per_level"]
    tp_pct = grid_params.get("take_profit_per_level_pct", 3)
    sl_pct = grid_params.get("portfolio_stop_loss_pct", 20)

    # Each grid level buys at a different price in the range
    # and takes profit tp_pct% above that level's buy price.
    # The overall portfolio stop is sl_pct%.

    return {
        "meta": {
            **meta,
            "description": meta.get("description", f"Grid: {levels} levels, ${price_min}-${price_max}"),
        },
        "instrument": instrument,
        "entry": {
            "type": "time_schedule",
            "interval": "1H",
        },
        "exit": {
            "stop_loss": {"pct": min(sl_pct, 20)},  # L3 capped at 20
            "take_profit": {"pct": tp_pct},
        },
        "sizing": {
            "type": "fixed_usd",
            "usd": usd_per_level,
        },
        "filters": [
            {
                "type": "price_range",
                "min_price": price_min,
                "max_price": price_max,
            },
        ],
        "risk_overlays": [
            {
                "type": "max_concurrent_positions",
                "n": min(levels, 10),  # schema max is 10
            },
        ],
    }
