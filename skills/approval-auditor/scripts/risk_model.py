"""
Risk scoring model for ERC-20 approvals.

Every approval is scored from 0 to 100 based on a weighted combination of
signals. Higher score = higher risk. The model is deterministic and auditable:
given the same input, it always produces the same output, and every point
contributing to the score is explained in `reasons`.

Score bands:
    80-100: CRITICAL  — revoke immediately
    50-79:  HIGH      — revoke soon
    30-49:  MEDIUM    — review and consider revoking
    0-29:   LOW       — informational

This module has zero network side effects. It consumes already-fetched data
and returns a score. All network I/O happens in audit.py.
"""

from __future__ import annotations

from typing import Any

# Allowances at or above this value are treated as effectively unlimited.
# The canonical sentinel is 2**256 - 1, but many wallets grant 2**255 or
# similarly huge values that are indistinguishable in practice.
NEAR_UNLIMITED_THRESHOLD = 2 ** 255


def score_approval(ctx: dict[str, Any]) -> dict[str, Any]:
    """
    Compute a risk score for a single approval.

    Input `ctx` keys:
        allowance_raw   (int)  current on-chain allowance in wei-units
        is_unlimited    (bool) allowance >= NEAR_UNLIMITED_THRESHOLD
        spender_is_eoa  (bool) spender has no bytecode (is a plain wallet)
        spender_known   (bool) spender is in the curated known-spender list
        spender_category (str) category if known, else empty
        is_bluechip     (bool) token is a blue-chip (USDC/USDT/DAI/WETH/WBTC)
        bluechip_symbol (str)  blue-chip symbol if applicable, else empty
        age_days        (int)  days since the last approval event
        last_allowance_raw (int) allowance granted by the latest Approval event

    Returns dict:
        score       (int 0-100)
        band        (str CRITICAL|HIGH|MEDIUM|LOW)
        reasons     (list[str]) human-readable list of signals that fired
    """
    score = 0
    reasons: list[str] = []

    is_unlimited = ctx.get("is_unlimited", False)
    spender_is_eoa = ctx.get("spender_is_eoa", False)
    spender_known = ctx.get("spender_known", False)
    is_bluechip = ctx.get("is_bluechip", False)
    bluechip_symbol = ctx.get("bluechip_symbol", "")
    age_days = int(ctx.get("age_days", 0))

    # ── Allowance magnitude ─────────────────────────────────────────────
    if is_unlimited:
        score += 40
        reasons.append("unlimited allowance (2^255 or higher)")

    # ── Spender identity ────────────────────────────────────────────────
    # A spender that is a plain EOA (not a contract) is the single strongest
    # red flag in approval auditing. It almost always means a phishing drain.
    if spender_is_eoa:
        score += 45
        reasons.append("spender is an externally-owned account, not a contract")

    # Unknown contract is not automatically bad, but it is worth noting when
    # combined with other signals.
    if not spender_known and not spender_is_eoa:
        score += 15
        reasons.append("spender not in curated known-protocol list")

    # ── Asset value at stake ────────────────────────────────────────────
    if is_bluechip:
        score += 10
        reasons.append(
            f"approval is on a blue-chip asset ({bluechip_symbol}) — higher value at stake"
        )

    # ── Dormancy ────────────────────────────────────────────────────────
    if age_days >= 365:
        score += 10
        reasons.append(f"approval has been dormant for {age_days} days")
    elif age_days >= 180:
        score += 5
        reasons.append(f"approval has been dormant for {age_days} days")

    # Clamp to 0-100
    if score < 0:
        score = 0
    if score > 100:
        score = 100

    band = _band(score)

    return {
        "score": score,
        "band": band,
        "reasons": reasons,
    }


def _band(score: int) -> str:
    if score >= 80:
        return "CRITICAL"
    if score >= 50:
        return "HIGH"
    if score >= 30:
        return "MEDIUM"
    return "LOW"
