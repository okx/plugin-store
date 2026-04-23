## Overview

A Python demonstration strategy that bets the direction of Polymarket's rolling 5-minute BTC Up/Down prediction markets based on short-term BTC price momentum from Binance public klines. Every buy submits through `polymarket-plugin` with `--strategy-id polymarket-btc-5min-momentum` so each trade is attributed to this strategy on the OKX backend.

## Prerequisites
- Python ≥ 3.9 (stdlib only — no pip installs required)
- `polymarket-plugin` ≥ 0.4.10 installed and on `PATH` (declared as `dependent_plugin`)
- An onchainos wallet with USDC.e on Polygon (≥ $3 recommended; Polymarket 5-min markets have a ~$2.50 minimum notional)
- Polymarket trading mode configured (`polymarket-plugin setup-proxy` for gasless, or EOA mode with POL for gas)
- Accessible region — Polymarket blocks the US and OFAC-sanctioned jurisdictions

## Quick Start
1. Install the dependent plugin (one-time): `npx skills add okx/plugin-store --skill polymarket-plugin --yes --global`
2. Verify the plugin works and is configured: `polymarket-plugin quickstart`
3. Preview the next slot's decision without placing a trade: `python3 strategy.py --dry-run`
4. Place a single bet on the next BTC 5-min slot: `python3 strategy.py --amount 2.5`
5. Run continuously (one decision per UTC 5-minute boundary): `python3 strategy.py --loop --amount 2.5`
6. Tune the signal: `--threshold 0.1` (fewer trades, stronger conviction), `--window 10` (shorter momentum lookback)
7. Track realized PnL on the OKX backend dashboard keyed by strategy ID `polymarket-btc-5min-momentum`
