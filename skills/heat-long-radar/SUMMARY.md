# heat-long-radar

## Overview

Heat Long Radar converts market heat, volume surge, negative funding, and OI expansion into an attributed Hyperliquid long workflow. It is adapted from `heat_long_radar.py`, but the public Skill does not include local Telegram configuration or runtime files.

Core operations:

- Detect heat from Binance Square, CoinGecko, and volume surge
- Confirm derivatives support through funding and OI
- Check whether the symbol exists on Hyperliquid
- Preview and execute through `hyperliquid-plugin`
- Attach `--strategy-id heat-long-radar` to every trading operation

Tags: `hyperliquid` `heat` `funding-rate` `open-interest` `long-strategy`

## Prerequisites

- `hyperliquid-plugin` installed
- Hyperliquid account ready via `hyperliquid quickstart`
- Candidate symbol listed on Hyperliquid
- User-provided notional, leverage, stop, and take-profit constraints

## Quick Start

1. Scan heat, volume, funding, and OI sources.
2. Keep only multi-factor long candidates.
3. Run `hyperliquid prices --coin <COIN>`.
4. Preview `hyperliquid order ... --strategy-id heat-long-radar`.
5. Execute with `--confirm` only after explicit confirmation.

