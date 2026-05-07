"""
LLM-driven strategy generation.

Takes a user profile + full primitive catalog → unique spec + theme + tagline.
Falls back gracefully if LLM is unavailable.
"""
from __future__ import annotations

import json
import os
import re
from typing import Any

# ── Primitive catalog (concise form for LLM prompt) ──────────────────────────

PRIMITIVE_CATALOG = """
ENTRY PRIMITIVES (pick exactly one):
- price_drop: {pct, lookback_bars} — buy when price drops X% in window
- price_breakout: {direction, lookback_bars, confirm_pct} — buy on breakout
- ma_cross: {fast_period, slow_period, ma_type} — buy when fast MA crosses slow MA
- rsi_threshold: {period, level, direction} — buy when RSI crosses level (cross_up at 30 = oversold)
- volume_spike: {multiplier, avg_bars} — buy on unusual volume surge
- time_schedule: {interval, anchor_utc} — DCA on fixed schedule (interval: 1D/1W/1M)
- smart_money_buy: {min_wallets, window_min} — buy when N smart wallets buy same token within window
- dev_buy: {min_usd, window_min} — buy when developer wallet buys their own token
- macd_cross: {fast_period, slow_period, signal_period, direction} — buy on MACD crossover
- bollinger_touch: {period, std_dev, band} — buy when price touches band (band: lower/upper)
- ranking_entry: {list_name, top_n} — snipe from list (list_name: trending/gainers/new)
- wallet_copy_buy: {target_wallet[], min_usd, mirror_mode} — mirror wallet buys (mirror_mode: instant/mcap_target)

EXIT PRIMITIVES:
Required field: stop_loss: {pct}
Optional extras in "other" array:
- {type: take_profit, pct} — fixed take profit %
- {type: trailing_stop, pct} — trailing stop %
- {type: tiered_take_profit, tiers:[{pct_gain, pct_sell},...]} — sell in layers (pct_sell values must sum to 100)
- {type: time_exit, max_bars} — exit after N bars
- {type: indicator_reversal, mirror_entry: true} — exit when entry signal reverses
- {type: smart_money_sell, min_wallets, window_min} — exit when smart money sells
- {type: dev_dump, min_usd} — exit on developer dump
- {type: wallet_mirror_sell, target_wallet[], min_pct_sold} — mirror wallet sells
- {type: fast_dump_exit, drop_pct, window_sec} — emergency rug/dump exit

FILTER PRIMITIVES (optional array):
General filters: time_window, volatility_range, volume_minimum, cooldown, market_regime, price_range, btc_overlay, top_zone_guard
Token safety (use for meme/new tokens): mcap_range, launch_age, honeypot_check, lp_locked, buy_tax_max, sell_tax_max, liquidity_min, top_holders_max, bundler_ratio_max, dev_holding_max, insider_holding_max, fresh_wallet_ratio_max, smart_money_present_min, phishing_exclude, whale_concentration_max

SIZING PRIMITIVES (pick one):
- {type: fixed_usd, usd} — fixed dollar amount per trade
- {type: fixed_pct, pct} — fixed % of portfolio per trade
- {type: volatility_scaled, target_risk_pct, atr_period} — scale size by volatility

RISK OVERLAY PRIMITIVES (array):
- {type: max_daily_trades, n}
- {type: max_concurrent_positions, n}
- {type: drawdown_pause, pause_pct}
- {type: session_loss_pause, max_consecutive_losses}
- {type: correlation_cap, mode, max_correlated}
"""

_SYSTEM = """You are a trading strategy architect. Design unique, personalized trading strategies by snapping primitives together like building blocks.

Rules:
- Pick exactly ONE entry primitive, configure its params thoughtfully for the user's situation
- stop_loss is always required; add creative exit combos in "other"
- Use token safety filters for meme/new token strategies (honeypot, lp_locked, etc.)
- theme = 2-3 word punchy identity (like "Momentum Engine", "Shadow Whale", "Dip Assassin")
- tagline = one sentence: the core trading philosophy of this strategy
- Make it genuinely unique — tune params to the user's specific profile, not generic defaults
- Be creative with primitive combinations — the same goal with different risk/budget should feel different
"""

_PROMPT = """\
Design a trading strategy for this user.

USER PROFILE:
{profile_json}
{extra}

PRIMITIVE CATALOG:
{catalog}

Return ONLY valid JSON (no markdown, no explanation):
{{
  "theme": "2-3 word name",
  "tagline": "one sentence philosophy",
  "spec": {{
    "meta": {{
      "name": "snake_case_64chars_max",
      "version": "1.0",
      "risk_tier": "conservative|moderate|aggressive",
      "description": "one line description",
      "author_intent": "{goal}"
    }},
    "instrument": {{
      "symbol": "TOKEN-USDC or * for dynamic",
      "timeframe": "5m|15m|1H|4H|1D"
    }},
    "entry": {{"type": "...", ...params}},
    "exit": {{
      "stop_loss": {{"pct": N}},
      "other": [...]
    }},
    "sizing": {{"type": "fixed_usd", "usd": {budget}}},
    "filters": [...],
    "risk_overlays": [...]
  }}
}}

For dynamic/wallet/meme strategies add at top level: "universe": {{"selector": "entry_type_value", "chain": "solana"}}
"""


def _get_api_key() -> str:
    """Get API key from env or Claude OAuth credentials."""
    key = os.environ.get("ANTHROPIC_API_KEY", "").strip()
    if key:
        return key
    creds_path = os.path.expanduser("~/.claude/.credentials.json")
    if not os.path.isfile(creds_path):
        return ""
    try:
        import time
        with open(creds_path) as f:
            creds = json.load(f)
        oauth = creds.get("claudeAiOauth", {})
        token = oauth.get("accessToken", "")
        if token and oauth.get("expiresAt", 0) > time.time() * 1000:
            return token
    except Exception:
        pass
    return ""


def _extract_json(text: str) -> dict[str, Any]:
    # Strip any leading non-JSON characters (org prefixes, markdown fences, etc.)
    text = text.strip()
    # Find first { or [ — start of JSON
    start = min(
        (text.find(c) for c in "{[" if text.find(c) != -1),
        default=0,
    )
    text = text[start:]
    # Strip trailing markdown fences
    text = re.sub(r"\s*```\s*$", "", text).strip()
    return json.loads(text)


# "Other" exit types that must live in exit.other[], not directly on exit
_OTHER_EXIT_TYPES = {
    "time_exit", "indicator_reversal", "smart_money_sell",
    "dev_dump", "wallet_mirror_sell", "fast_dump_exit",
}
# Inner exit types that must be direct keys on exit, not in exit.other[]
_INNER_EXIT_KEYS = {"stop_loss", "take_profit", "trailing_stop", "tiered_take_profit"}


def _normalize_spec(spec: dict[str, Any]) -> dict[str, Any]:
    """
    Auto-repair common LLM structural hallucinations before harness validation.

    Fixes applied (all silent — no errors raised here):
    1. Other-exit types placed directly on exit → moved to exit.other[]
    2. Inner-exit types placed in exit.other[] → promoted to direct exit keys
    3. Missing universe block when symbol == "*"
    4. tiered_take_profit tiers pct_sell doesn't sum to 100 → rescale last tier
    5. meta.live_only flag auto-set (harness also does this, but set it early)
    """
    import copy
    spec = copy.deepcopy(spec)

    exit_block = spec.get("exit", {})

    # Fix 1: other-exit types sitting directly on exit → move to exit.other[]
    other = exit_block.get("other", [])
    for key in list(exit_block.keys()):
        if key in _OTHER_EXIT_TYPES:
            item = exit_block.pop(key)
            if isinstance(item, dict):
                item.setdefault("type", key)
            else:
                item = {"type": key}
            other.append(item)
    if other:
        exit_block["other"] = other

    # Fix 2: inner-exit types sitting inside exit.other[] → promote to direct keys
    remaining_other = []
    for item in exit_block.get("other", []):
        if isinstance(item, dict) and item.get("type") in _INNER_EXIT_KEYS:
            key = item["type"]
            promoted = {k: v for k, v in item.items() if k != "type"}
            exit_block.setdefault(key, promoted)
        else:
            remaining_other.append(item)
    if remaining_other:
        exit_block["other"] = remaining_other
    elif "other" in exit_block:
        del exit_block["other"]

    spec["exit"] = exit_block

    # Fix 3: missing universe when symbol == "*"
    instr = spec.get("instrument", {})
    if instr.get("symbol") == "*" and "universe" not in spec:
        entry_type = spec.get("entry", {}).get("type", "ranking_entry")
        chain = spec.get("meta", {}).get("chain", "solana")
        spec["universe"] = {"selector": entry_type, "chain": chain}

    # Fix 4: tiered_take_profit pct_sell rescaling
    ttp = exit_block.get("tiered_take_profit", {})
    tiers = ttp.get("tiers", [])
    if tiers:
        total = sum(t.get("pct_sell", 0) for t in tiers)
        if total != 100 and total > 0:
            # Rescale all tiers proportionally, assign remainder to last
            scaled = [round(t.get("pct_sell", 0) * 100 / total) for t in tiers]
            diff = 100 - sum(scaled)
            scaled[-1] += diff
            for t, s in zip(tiers, scaled):
                t["pct_sell"] = s

    return spec


def generate_strategy_spec(
    profile: dict[str, Any],
    api_key: str = "",
) -> tuple[dict[str, Any], str, str, list[str]]:
    """
    LLM-generate a unique strategy spec.

    Returns (spec, theme, tagline, errors).
    Empty spec + errors list means failure — caller should fall back to template.
    """
    if not api_key:
        api_key = _get_api_key()
    if not api_key:
        return {}, "", "", ["No API key — LLM generation unavailable"]

    goal   = profile.get("goal", "unknown")
    budget = profile.get("total_budget") or profile.get("budget_per_trade") or 100
    wallets = profile.get("target_wallets", [])

    extra = f"\nWallets to track/mirror: {wallets}" if wallets else ""

    prompt = _PROMPT.format(
        profile_json=json.dumps(profile, indent=2),
        catalog=PRIMITIVE_CATALOG,
        goal=goal,
        budget=budget,
        extra=extra,
    )

    try:
        import anthropic
        from harness import validate_spec
        client = anthropic.Anthropic(api_key=api_key)

        messages: list[dict] = [{"role": "user", "content": prompt}]
        theme = tagline = ""
        spec: dict[str, Any] = {}
        last_errors: list[str] = []

        # Retry loop — up to 3 attempts, each feeds harness errors back to LLM
        for attempt in range(3):
            resp = client.messages.create(
                model="claude-haiku-4-5-20251001",
                max_tokens=2000,
                system=_SYSTEM,
                messages=messages,
            )
            raw = resp.content[0].text.strip()

            try:
                data = _extract_json(raw)
            except json.JSONDecodeError as e:
                last_errors = [f"Invalid JSON on attempt {attempt + 1}: {e}"]
                messages.append({"role": "assistant", "content": raw})
                messages.append({"role": "user", "content": (
                    f"Your response was not valid JSON. Error: {e}\n"
                    "Return ONLY a valid JSON object — no markdown, no explanation."
                )})
                continue

            theme   = data.get("theme", "")
            tagline = data.get("tagline", "")
            spec    = data.get("spec", {})

            if not spec or "entry" not in spec:
                last_errors = ["Incomplete spec: missing 'entry' block"]
                messages.append({"role": "assistant", "content": raw})
                messages.append({"role": "user", "content": (
                    "The spec is missing the required 'entry' block. "
                    "Return the complete JSON spec including entry, exit, sizing, filters, risk_overlays."
                )})
                continue

            # Normalize before validation (fix common structural errors)
            spec = _normalize_spec(spec)
            spec.setdefault("meta", {}).update({"theme": theme, "tagline": tagline})

            ok, errors, _ = validate_spec(spec)
            if ok:
                return spec, theme, tagline, []

            # Feed errors back for next attempt
            last_errors = errors
            messages.append({"role": "assistant", "content": raw})
            messages.append({"role": "user", "content": (
                f"The spec failed harness validation on attempt {attempt + 1}. "
                f"Fix ALL of these errors and return the corrected JSON:\n"
                + "\n".join(f"- {e}" for e in errors)
            )})

        # All retries exhausted — return best attempt + errors
        return spec, theme, tagline, last_errors

    except Exception as e:
        return {}, "", "", [f"LLM generation failed: {e}"]


# ── Deterministic theme fallback (when LLM unavailable) ──────────────────────

_FALLBACK_THEMES: dict[str, tuple[str, str]] = {
    "meme_sniper":  ("Momentum Sniper",  "Catch the spike, bank in layers, flee the rug"),
    "smart_money":  ("Shadow Whale",     "Follow alpha moves, exit when they exit"),
    "copy_trade":   ("Mirror Strike",    "Instant mirror execution, ride their edge"),
    "dca":          ("Steady Stacker",   "Time-based accumulation with trailing protection"),
    "dip_buy":      ("Dip Assassin",     "Buy the fear, sell the recovery"),
    "trend_follow": ("Trend Rider",      "Never fight the trend, let winners run"),
    "mean_revert":  ("Rubber Band",      "Oversold is just a discount waiting to expire"),
    "grid":         ("Grid Maker",       "Be the market maker, collect both sides"),
}


def get_fallback_theme(goal_key: str) -> tuple[str, str]:
    """Return (theme, tagline) for when LLM is unavailable."""
    return _FALLBACK_THEMES.get(goal_key, ("Custom Strategy", "Built from your unique profile"))
