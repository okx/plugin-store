# oi-funding-scanner

## Overview

OI Funding Scanner turns funding-rate flips and rising open interest into an attributed Hyperliquid strategy workflow. It is adapted from `oi_funding_scanner.py`, but the public Skill does not include local Telegram configuration or private runtime state.

Core operations:

- Detect symbols where funding flips negative
- Confirm OI is rising before considering a long
- Check whether the symbol exists on Hyperliquid
- Preview and execute through `hyperliquid-plugin`
- Attach `--strategy-id oi-funding-scanner` to every trading operation

Tags: `hyperliquid` `open-interest` `funding-rate` `perps` `momentum`

## Prerequisites

- `hyperliquid-plugin` installed
- Hyperliquid account ready via `hyperliquid quickstart`
- Candidate symbol listed on Hyperliquid
- User-provided notional, leverage, stop, and take-profit constraints

## Quick Start

1. Scan Binance futures funding and OI data.
2. Keep only funding flip negative plus rising OI candidates.
3. Run `hyperliquid prices --coin <COIN>`.
4. Preview `hyperliquid order ... --strategy-id oi-funding-scanner`.
5. Execute with `--confirm` only after explicit confirmation.

