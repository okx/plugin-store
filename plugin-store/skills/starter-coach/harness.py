"""
Harness — validate strategy specs against schema.json + forbidden-pattern checks.

Usage:
    from harness import validate_spec
    ok, errors = validate_spec(spec_dict)
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

try:
    import jsonschema
    from jsonschema import Draft202012Validator
except ImportError:
    jsonschema = None  # type: ignore
    Draft202012Validator = None  # type: ignore

SCHEMA_PATH = Path(__file__).parent / "schema.json"

# Primitives with x-live-only: true in schema.json
LIVE_ONLY_TYPES: set[str] = {
    # entries
    "smart_money_buy", "dev_buy", "ranking_entry", "wallet_copy_buy",
    # exits
    "smart_money_sell", "dev_dump", "wallet_mirror_sell", "fast_dump_exit",
    # filters
    "mcap_range", "launch_age",
    "honeypot_check", "lp_locked", "buy_tax_max", "sell_tax_max",
    "liquidity_min", "top_holders_max", "bundler_ratio_max",
    "dev_holding_max", "insider_holding_max", "fresh_wallet_ratio_max",
    "smart_money_present_min", "phishing_exclude", "whale_concentration_max",
}


def _load_schema() -> dict:
    with open(SCHEMA_PATH) as f:
        return json.load(f)


def _collect_types(spec: dict) -> set[str]:
    """Walk the spec and collect every primitive type used."""
    types: set[str] = set()
    entry = spec.get("entry", {})
    if isinstance(entry, dict) and "type" in entry:
        types.add(entry["type"])
    exit_block = spec.get("exit", {})
    for inner_key in ("stop_loss", "take_profit", "trailing_stop", "tiered_take_profit"):
        if inner_key in exit_block:
            types.add(inner_key)
    for other in exit_block.get("other", []):
        if isinstance(other, dict) and "type" in other:
            types.add(other["type"])
    for filt in spec.get("filters", []):
        if isinstance(filt, dict) and "type" in filt:
            types.add(filt["type"])
    sizing = spec.get("sizing", {})
    if isinstance(sizing, dict) and "type" in sizing:
        types.add(sizing["type"])
    for overlay in spec.get("risk_overlays", []):
        if isinstance(overlay, dict) and "type" in overlay:
            types.add(overlay["type"])
    return types


def _check_live_only(spec: dict) -> bool:
    """Return True if any primitive in the spec is live_only."""
    return bool(_collect_types(spec) & LIVE_ONLY_TYPES)


def _check_h01(spec: dict, errors: list[str]) -> None:
    """H-01: stop_loss is required."""
    exit_block = spec.get("exit", {})
    if "stop_loss" not in exit_block:
        errors.append("H-01: exit.stop_loss is required. A strategy without a stop is gambling.")
    elif not isinstance(exit_block["stop_loss"], dict) or "pct" not in exit_block["stop_loss"]:
        errors.append("H-01: exit.stop_loss must have a 'pct' field.")


def _check_h02(spec: dict, errors: list[str]) -> None:
    """H-02: No martingale / averaging down."""
    meta = spec.get("meta", {})
    for field in ("description", "author_intent", "name"):
        text = str(meta.get(field, "")).lower()
        for keyword in ("martingale", "double down", "average down", "averaging down"):
            if keyword in text:
                errors.append(f"H-02: Martingale / averaging down detected in meta.{field}. Rejected.")
                return


def _check_h03(spec: dict, errors: list[str]) -> None:
    """H-03: stop_loss.pct must be < take_profit.pct (when take_profit is used)."""
    exit_block = spec.get("exit", {})
    sl = exit_block.get("stop_loss", {})
    tp = exit_block.get("take_profit", {})
    if not sl or not tp:
        return
    sl_pct = sl.get("pct", 0)
    tp_pct = tp.get("pct", 0)
    if sl_pct and tp_pct and sl_pct >= tp_pct:
        errors.append(
            f"H-03: stop_loss ({sl_pct}%) must be tighter than take_profit ({tp_pct}%). "
            "Negative asymmetry rejected."
        )


def _check_h05(spec: dict, errors: list[str]) -> None:
    """H-05: sizing.pct * max_daily_trades.n <= 20% daily risk cap."""
    sizing = spec.get("sizing", {})
    if sizing.get("type") != "fixed_pct":
        return
    pct = sizing.get("pct", 0)
    for overlay in spec.get("risk_overlays", []):
        if overlay.get("type") == "max_daily_trades":
            n = overlay.get("n", 1)
            daily_risk = pct * n
            if daily_risk > 20:
                errors.append(
                    f"H-05: fixed_pct ({pct}%) x max_daily_trades ({n}) = {daily_risk}% "
                    "exceeds 20% daily risk cap."
                )
            return


def _check_h06(spec: dict, errors: list[str]) -> None:
    """H-06: No unknown primitive types."""
    known_types = {
        "price_drop", "price_breakout", "ma_cross", "rsi_threshold",
        "volume_spike", "time_schedule", "smart_money_buy", "dev_buy",
        "macd_cross", "bollinger_touch", "ranking_entry", "wallet_copy_buy",
        "time_exit", "indicator_reversal", "smart_money_sell", "dev_dump",
        "wallet_mirror_sell", "fast_dump_exit",
        "time_window", "volatility_range", "volume_minimum", "cooldown",
        "market_regime", "price_range", "btc_overlay", "top_zone_guard",
        "mcap_range", "launch_age",
        "honeypot_check", "lp_locked", "buy_tax_max", "sell_tax_max",
        "liquidity_min", "top_holders_max", "bundler_ratio_max",
        "dev_holding_max", "insider_holding_max", "fresh_wallet_ratio_max",
        "smart_money_present_min", "phishing_exclude", "whale_concentration_max",
        "fixed_pct", "fixed_usd", "volatility_scaled",
        "max_daily_trades", "max_concurrent_positions", "drawdown_pause",
        "correlation_cap", "session_loss_pause",
    }
    for t in _collect_types(spec):
        if t in ("stop_loss", "take_profit", "trailing_stop", "tiered_take_profit"):
            continue
        if t not in known_types:
            errors.append(f"H-06: Unknown primitive type '{t}'. No freeform code allowed.")


def validate_spec(spec: dict[str, Any]) -> tuple[bool, list[str], dict[str, Any]]:
    """
    Validate a strategy spec.

    Returns:
        (ok, errors, meta)
        - ok: True if spec is valid
        - errors: list of human-readable error strings
        - meta: {"live_only": bool, "primitive_count": int, "types_used": list[str]}
    """
    errors: list[str] = []

    # 1. JSON Schema validation (H-04 param bounds + structural)
    if Draft202012Validator is not None:
        schema = _load_schema()
        validator = Draft202012Validator(schema)
        for error in sorted(validator.iter_errors(spec), key=lambda e: list(e.path)):
            path = ".".join(str(p) for p in error.absolute_path) or "(root)"
            errors.append(f"Schema: {path} — {error.message}")
    else:
        errors.append("Warning: jsonschema not installed. Run: pip install jsonschema")

    # 2. Forbidden-pattern checks
    _check_h01(spec, errors)
    _check_h02(spec, errors)
    _check_h03(spec, errors)
    _check_h05(spec, errors)
    _check_h06(spec, errors)

    # 3. Derive metadata
    types_used = sorted(_collect_types(spec))
    live_only = _check_live_only(spec)

    meta = {
        "live_only": live_only,
        "primitive_count": len(types_used),
        "types_used": types_used,
    }

    return len(errors) == 0, errors, meta


if __name__ == "__main__":
    import sys
    if len(sys.argv) < 2:
        print("Usage: python harness.py <spec.json>")
        sys.exit(1)
    with open(sys.argv[1]) as f:
        spec = json.load(f)
    ok, errors, meta = validate_spec(spec)
    if ok:
        print(f"PASS  live_only={meta['live_only']}  primitives={meta['primitive_count']}")
        print(f"  types: {', '.join(meta['types_used'])}")
    else:
        print(f"FAIL  ({len(errors)} errors)")
        for e in errors:
            print(f"  - {e}")
        sys.exit(1)
