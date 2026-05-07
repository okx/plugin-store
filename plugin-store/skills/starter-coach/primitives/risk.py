"""
Risk overlay evaluators — 5 primitives.

Each function: check_overlay(state, params) -> bool
  True = OK to proceed, False = blocked by overlay.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any
import time as _time


@dataclass
class PortfolioState:
    """Tracked by the engine across trades."""
    trades_today: int = 0
    open_positions: int = 0
    open_tokens: list[str] = field(default_factory=list)
    equity_usd: float = 0.0
    peak_equity_usd: float = 0.0
    consecutive_losses: int = 0
    session_start_ts: float = 0.0


def check_max_daily_trades(state: PortfolioState, params: dict) -> bool:
    """R-01: Hard cap on entries per 24h."""
    return state.trades_today < params["n"]


def check_max_concurrent_positions(state: PortfolioState, params: dict) -> bool:
    """R-02: Max open positions at once."""
    return state.open_positions < params["n"]


def check_drawdown_pause(state: PortfolioState, params: dict) -> bool:
    """R-03: Pause if equity drawdown exceeds threshold."""
    pause_pct = params["pause_pct"]
    if state.peak_equity_usd == 0:
        return True
    dd = (state.peak_equity_usd - state.equity_usd) / state.peak_equity_usd * 100
    return dd < pause_pct


def check_correlation_cap(state: PortfolioState, params: dict) -> bool:
    """R-04: v1.0 same_token_dedupe only. Prevent duplicate token positions."""
    max_corr = params["max_correlated"]
    # Count how many open positions share the same token
    from collections import Counter
    counts = Counter(state.open_tokens)
    for token, count in counts.items():
        if count >= max_corr:
            return False
    return True


def check_session_loss_pause(state: PortfolioState, params: dict) -> bool:
    """R-05: Pause after N consecutive losses."""
    max_losses = params["max_consecutive_losses"]
    session_hours = params.get("session_hours", 24)
    # Check if we're still in the session window
    now = _time.time()
    if now - state.session_start_ts > session_hours * 3600:
        # Session expired, reset
        state.consecutive_losses = 0
        state.session_start_ts = now
        return True
    return state.consecutive_losses < max_losses


RISK_EVALUATORS: dict[str, Any] = {
    "max_daily_trades": check_max_daily_trades,
    "max_concurrent_positions": check_max_concurrent_positions,
    "drawdown_pause": check_drawdown_pause,
    "correlation_cap": check_correlation_cap,
    "session_loss_pause": check_session_loss_pause,
}


def check_all_overlays(
    state: PortfolioState, overlays: list[dict]
) -> tuple[bool, list[str]]:
    """
    Check all risk overlays. ALL must pass.
    Returns (all_pass, list_of_blocking_overlay_types).
    """
    blockers: list[str] = []
    for overlay in overlays:
        otype = overlay.get("type", "")
        checker = RISK_EVALUATORS.get(otype)
        if checker is None:
            blockers.append(f"unknown:{otype}")
            continue
        params = {k: v for k, v in overlay.items() if k != "type"}
        if not checker(state, params):
            blockers.append(otype)
    return len(blockers) == 0, blockers
