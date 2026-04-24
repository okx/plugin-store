# pm-5m-momentum-sniper

## Overview

Momentum-driven trading skill for Polymarket 5-minute crypto Up/Down markets. Analyzes recent resolved rounds to generate a directional signal, then executes trades only when the signal aligns with favorable odds. Supports continuous multi-round sessions with built-in risk controls.

Core operations:

- Signal engine binary (`pm-5m-signal`) fetches real-time 1m klines from Binance, computes RSI/EMA/momentum/volume indicators
- Half-Kelly position sizing based on signal confidence and market odds
- Automated direction recommendation (Up/Down) with confidence scoring
- Continuous session loop with per-round user confirmation
- Risk management: stop-loss, take-profit, consecutive loss pause, daily loss limit

Best during volatile markets (US open/close, news events, trending moves). Skips trades during sideways conditions by design.

Tags: `polymarket` `prediction-market` `crypto` `momentum` `five-minute` `kelly-criterion`

## Prerequisites

- Install basic skill: `npx skills add okx/plugin-store --skill polymarket-plugin` (v0.4.10+)
- onchainos CLI installed and logged in: `onchainos wallet login your@email.com`
- Polygon wallet with USDC.e (collateral) and small amount of POL (gas)
- Polymarket access (jurisdiction check via `polymarket-plugin check-access`)
- Supported chains: Polygon (chain 137)
- Recommended: POLY_PROXY mode for gas-free trading (`polymarket-plugin setup-proxy --confirm`)
- Minimum balance: 30 USDC recommended for a session

## Quick Start

1. **Install dependencies**: `npx skills add okx/plugin-store --skill polymarket-plugin`
2. **Login wallet**: `onchainos wallet login your@email.com` and verify with `onchainos wallet addresses --chain 137`
3. **Check access**: Run `polymarket-plugin check-access` to verify your jurisdiction is supported.
4. **Setup proxy**: Run `polymarket-plugin setup-proxy --confirm` for gasless trading.
5. **Fund proxy**: Run `polymarket-plugin deposit --amount 50 --confirm` to move USDC.e into trading account.
6. **Start a session**: Tell the agent "Run momentum sniper on BTC" — it handles signal generation, odds filtering, and trade execution with your confirmation each round.
