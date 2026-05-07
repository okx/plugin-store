# market-structure-analyzer

## Overview

Crypto market-structure research agent with 24+ indicators across derivatives, options (gamma wall, skew), on-chain (MVRV, smart money, DEX hot tokens), and macro sentiment. Live auto-refreshing dashboard with K-line candlestick charts, TA overlays, and 12-signal composite scoring.

Core operations:

- Fetch and analyze derivatives positioning (funding, OI, basis, long/short)
- Display options flow data (gamma wall, 25-delta skew, ATM IV, butterfly)
- Track on-chain metrics (MVRV, smart money signals, DEX hot tokens)
- Compute composite score from 12 weighted signals (-100 to +100)
- Render live K-line charts with RSI, MACD, Bollinger Bands overlays

Tags: `derivatives` `options` `on-chain` `market-structure` `mvrv` `smart-money` `live-dashboard`

## Prerequisites

- Python 3.8+ (stdlib only, no pip dependencies)
- OKX CeFi CLI installed (`npm install -g @okx_ai/okx-trade-cli`) — no API key needed
- OnchainOS CLI (`onchainos`) at `~/.local/bin/onchainos` — for smart money signals and DEX hot tokens

## Quick Start

1. **Start the live dashboard**: Run `python3 msa_server.py` from the skill directory. The dashboard opens at `http://localhost:8420` with auto-refreshing charts and indicators.

2. **Select token and timeframe**: Click token buttons (BTC, ETH, SOL, etc.) and timeframe buttons (5m, 15m, 1H, 4H, 1D) to switch the chart and indicator panels.

3. **Read the composite score**: The header shows a score from -100 to +100 (BULLISH / NEUTRAL / BEARISH) computed from 12 weighted signals including funding rate, OI delta, RSI, MACD, Fear & Greed, and smart money flow.

4. **CLI-only mode** (optional): Run `python3 scripts/fetch_market_data.py BTC ETH SOL` to get raw JSON output without the dashboard.
