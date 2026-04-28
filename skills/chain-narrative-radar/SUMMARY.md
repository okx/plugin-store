# chain-narrative-radar

## Overview

Chain Narrative Radar turns fresh on-chain narrative discovery into an attributed Hyperliquid strategy workflow. It is adapted from a local scanner that tracked DEXScreener, Pump.fun metadata, RugCheck, and GoPlus safety data, but this public Skill does not include private runtime files or Telegram configuration.

Core operations:

- Detect new narrative clusters and repeated momentum signals
- Filter spam, duplicate, and unsafe token themes
- Check whether a candidate symbol exists on Hyperliquid
- Preview and execute only through `hyperliquid-plugin`
- Attach `--strategy-id chain-narrative-radar` to every trading operation

Tags: `hyperliquid` `narrative` `momentum` `dexscreener` `trading-strategy`

## Prerequisites

- `hyperliquid-plugin` installed
- Hyperliquid account ready via `hyperliquid quickstart`
- User-provided max notional and risk preference
- Candidate symbol must be listed on Hyperliquid before any live trade

## Quick Start

1. Run `hyperliquid quickstart`.
2. Build a narrative candidate list from public DEXScreener/Pump.fun/RugCheck/GoPlus data.
3. Run `hyperliquid prices --coin <COIN>` for the candidate.
4. Preview `hyperliquid order ... --strategy-id chain-narrative-radar`.
5. Execute with `--confirm` only after explicit confirmation.

