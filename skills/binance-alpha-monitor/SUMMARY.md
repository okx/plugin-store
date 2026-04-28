# binance-alpha-monitor

## Overview

Binance Alpha Monitor converts Binance Alpha/listing-style announcements into an attributed Hyperliquid event-trading workflow. It is adapted from `binance_alpha_monitor.py`, but the public Skill removes local Telegram configuration and optional private LLM extraction.

Core operations:

- Monitor Binance public announcements for Alpha, airdrop, TGE, launch, and listing events
- Exclude delistings, maintenance, and generic market updates
- Enrich candidates with public CoinGecko data
- Check whether the symbol exists on Hyperliquid
- Preview and execute through `hyperliquid-plugin`
- Attach `--strategy-id binance-alpha-monitor` to every trading operation

Tags: `hyperliquid` `binance-alpha` `listing-monitor` `event-trading` `narrative`

## Prerequisites

- `hyperliquid-plugin` installed
- Hyperliquid account ready via `hyperliquid quickstart`
- Candidate symbol listed on Hyperliquid
- User-provided notional, leverage, stop, take-profit, and max hold-time constraints

## Quick Start

1. Monitor Binance public announcement data.
2. Keep only valid Alpha/listing-style events.
3. Run `hyperliquid prices --coin <COIN>`.
4. Preview `hyperliquid order ... --strategy-id binance-alpha-monitor`.
5. Execute with `--confirm` only after explicit confirmation.

