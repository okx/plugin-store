"""
一键发币 v1.0 — Launchpad adapter base class.
All launchpad adapters inherit from this and implement launch().
"""
from __future__ import annotations

import abc
import os
from dataclasses import dataclass, field
from typing import Optional


def onchainos_bin() -> str:
    """Resolve the onchainos CLI binary path."""
    env = os.environ.get("ONCHAINOS_BIN", "")
    if env:
        return env
    home = os.path.expanduser("~/.local/bin/onchainos")
    if os.path.isfile(home):
        return home
    return "onchainos"  # fallback to PATH


@dataclass
class LaunchParams:
    """Parameters collected from user for token launch."""

    # ── Required ──────────────────────────────────────────────────────
    name: str                          # Token name
    symbol: str                        # Token ticker
    description: str                   # Token description
    image_path: str                    # Local file path or URL

    # ── Optional socials ──────────────────────────────────────────────
    website: str = ""
    twitter: str = ""
    telegram: str = ""

    # ── Launchpad ─────────────────────────────────────────────────────
    launchpad: str = "pumpfun"         # Launchpad adapter name

    # ── Bundled buy ───────────────────────────────────────────────────
    buy_amount: float = 0.0            # Native token amount (0 = create only)
    slippage_bps: int = 1000           # Slippage in basis points (1000 = 10%)
    mev_protection: bool = True        # Jito bundle / MEV protection

    # ── Wallet (resolved at runtime) ──────────────────────────────────
    wallet_address: str = ""

    # ── IPFS (resolved during upload) ─────────────────────────────────
    image_cid: str = ""                # Set after image upload
    metadata_cid: str = ""             # Set after metadata upload
    metadata_uri: str = ""             # Full URI for on-chain metadata

    # ── Launchpad-specific extras ─────────────────────────────────────
    extras: dict = field(default_factory=dict)
    # Examples:
    #   pumpfun: {"priority_fee": 0.0005, "tip_fee": 0.0001, "pool": "pump"}
    #   bags:    {"fee_claimers": [...]}
    #   flap:    {"buy_tax": 500, "sell_tax": 300, "migrator": 1}
    #   fourmeme: {"category": "Meme"}


@dataclass
class LaunchResult:
    """Result returned after a successful token launch."""

    success: bool
    token_address: str = ""
    tx_hash: str = ""
    explorer_url: str = ""
    trade_page_url: str = ""
    error: str = ""
    tokens_received: float = 0.0       # If bundled buy, how many tokens received
    raw_response: dict = field(default_factory=dict)


class LaunchpadAdapter(abc.ABC):
    """Abstract base class for all launchpad adapters."""

    @property
    @abc.abstractmethod
    def name(self) -> str:
        """Launchpad identifier (e.g. 'pumpfun')."""
        ...

    @property
    @abc.abstractmethod
    def display_name(self) -> str:
        """Human-readable name (e.g. 'pump.fun')."""
        ...

    @property
    @abc.abstractmethod
    def chain(self) -> str:
        """Chain identifier: 'solana' or 'bsc'."""
        ...

    @abc.abstractmethod
    async def launch(self, params: LaunchParams) -> LaunchResult:
        """Execute the token launch.

        The caller is responsible for:
        - Uploading image + metadata to IPFS (sets params.image_cid, metadata_cid, metadata_uri)
        - Resolving wallet address (sets params.wallet_address)
        - Checking balance sufficiency

        The adapter is responsible for:
        - Building the launch transaction(s)
        - Signing via onchainos wallet
        - Submitting to chain
        - Returning the result
        """
        ...

    def estimate_cost(self, params: LaunchParams) -> float:
        """Estimate total cost in native token (buy_amount + fees + gas)."""
        return params.buy_amount + self._fee_estimate(params)

    def _fee_estimate(self, params: LaunchParams) -> float:
        """Override in subclass for launchpad-specific fee estimates."""
        return 0.02  # Default: small buffer for gas/rent
