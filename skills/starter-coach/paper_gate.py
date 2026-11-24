"""
Paper-trade graduation gate for live_only strategies.

Tracks per-strategy:
  - paper_count: completed paper trades
  - live_micro_count: completed live micro-trades (small size)
  - days_observed: calendar days since first paper trade
  - harness_breaches: number of times harness caught a violation

Unlocks full sizing when:
  >= 10 paper trades + >= 5 live micro-trades + >= 7 days + 0 harness breaches
"""
from __future__ import annotations

import json
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any


GATE_DIR = Path(__file__).parent / ".paper_gate"

# Graduation thresholds
MIN_PAPER_TRADES = 10
MIN_LIVE_MICRO_TRADES = 5
MIN_DAYS_OBSERVED = 7
MAX_HARNESS_BREACHES = 0


@dataclass
class GateState:
    """Persisted state for a single strategy's graduation progress."""
    strategy_name: str = ""
    paper_count: int = 0
    live_micro_count: int = 0
    first_paper_ts: float = 0.0  # unix seconds
    harness_breaches: int = 0
    graduated: bool = False
    graduated_ts: float = 0.0

    @property
    def days_observed(self) -> float:
        if self.first_paper_ts == 0:
            return 0.0
        return (time.time() - self.first_paper_ts) / 86400

    @property
    def ready(self) -> bool:
        return (
            self.paper_count >= MIN_PAPER_TRADES
            and self.live_micro_count >= MIN_LIVE_MICRO_TRADES
            and self.days_observed >= MIN_DAYS_OBSERVED
            and self.harness_breaches <= MAX_HARNESS_BREACHES
        )

    def progress_summary(self) -> dict[str, Any]:
        return {
            "paper_trades": f"{self.paper_count}/{MIN_PAPER_TRADES}",
            "live_micro_trades": f"{self.live_micro_count}/{MIN_LIVE_MICRO_TRADES}",
            "days_observed": f"{self.days_observed:.1f}/{MIN_DAYS_OBSERVED}",
            "harness_breaches": self.harness_breaches,
            "graduated": self.graduated,
            "ready_to_graduate": self.ready,
        }


def _state_path(strategy_name: str) -> Path:
    GATE_DIR.mkdir(parents=True, exist_ok=True)
    return GATE_DIR / f"{strategy_name}.json"


def load_state(strategy_name: str) -> GateState:
    path = _state_path(strategy_name)
    if not path.exists():
        return GateState(strategy_name=strategy_name)
    with open(path) as f:
        data = json.load(f)
    return GateState(**data)


def save_state(state: GateState) -> None:
    path = _state_path(state.strategy_name)
    with open(path, "w") as f:
        json.dump(asdict(state), f, indent=2)


def record_paper_trade(strategy_name: str) -> GateState:
    """Record a completed paper trade."""
    state = load_state(strategy_name)
    if state.first_paper_ts == 0:
        state.first_paper_ts = time.time()
    state.paper_count += 1
    save_state(state)
    return state


def record_live_micro_trade(strategy_name: str) -> GateState:
    """Record a completed live micro-trade."""
    state = load_state(strategy_name)
    state.live_micro_count += 1
    save_state(state)
    return state


def record_harness_breach(strategy_name: str) -> GateState:
    """Record a harness violation during paper/micro trading."""
    state = load_state(strategy_name)
    state.harness_breaches += 1
    save_state(state)
    return state


def check_graduation(strategy_name: str) -> tuple[bool, dict[str, Any]]:
    """
    Check if strategy is ready to graduate to full sizing.

    Returns:
        (graduated, progress_summary)
    """
    state = load_state(strategy_name)
    if state.graduated:
        return True, state.progress_summary()
    if state.ready:
        state.graduated = True
        state.graduated_ts = time.time()
        save_state(state)
        return True, state.progress_summary()
    return False, state.progress_summary()


def get_allowed_size_multiplier(strategy_name: str) -> float:
    """
    Returns sizing multiplier:
      - 0.0 if no paper trades yet (blocked)
      - 0.1 during micro-trade phase (10% of spec size)
      - 1.0 if graduated (full size)
    """
    state = load_state(strategy_name)
    if state.graduated:
        return 1.0
    if state.paper_count >= MIN_PAPER_TRADES:
        return 0.1  # micro-trade phase
    return 0.0  # paper-only phase
