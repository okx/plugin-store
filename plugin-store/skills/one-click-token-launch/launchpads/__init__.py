"""Launchpad adapters for token creation."""
from __future__ import annotations

from .base import LaunchpadAdapter, LaunchParams, LaunchResult, onchainos_bin
from .pumpfun import PumpFunAdapter
from .bags import BagsAdapter
from .letsbonk import LetsBonkAdapter
from .moonit import MoonitAdapter
from .fourmeme import FourMemeAdapter
from .flap import FlapAdapter

ADAPTERS = {
    "pumpfun":  PumpFunAdapter,
    "bags":     BagsAdapter,
    "letsbonk": LetsBonkAdapter,
    "moonit":   MoonitAdapter,
    "fourmeme": FourMemeAdapter,
    "flap":     FlapAdapter,
}

__all__ = [
    "ADAPTERS",
    "LaunchpadAdapter",
    "LaunchParams",
    "LaunchResult",
    "PumpFunAdapter",
    "BagsAdapter",
    "LetsBonkAdapter",
    "MoonitAdapter",
    "FourMemeAdapter",
    "FlapAdapter",
    "get_adapter",
]


def get_adapter(launchpad: str) -> LaunchpadAdapter:
    """Get the adapter instance for a given launchpad name."""
    cls = ADAPTERS.get(launchpad)
    if cls is None:
        supported = ", ".join(ADAPTERS.keys())
        raise ValueError(f"Unknown launchpad: {launchpad}. Supported: {supported}")
    return cls()
