## Overview

Mainstream Spot Order is a multi-chain DEX spot trading system that runs a 6-signal ensemble (Momentum, EMA, RSI, MACD, Bollinger Bands, BTC Overlay) on 15-minute bars across 6 mainstream token pairs with AI-driven strategy optimization.

Core operations:

- Collect 15-minute OHLCV data for SOL, ETH, BTC, BNB, AVAX, and DOGE pairs
- Run AI-powered auto-research to optimize signal parameters per pair
- Backtest strategies with per-pair performance metrics
- Execute DEX spot trades via onchainos Agentic Wallet (TEE signing)
- Monitor live positions and signals on a web dashboard

Tags: `spot-trading` `solana` `ethereum` `bsc` `avalanche` `onchainos` `auto-research`

## Prerequisites

- No IP/region restrictions
- Supported chains: Solana, Ethereum, BSC, Avalanche
- Supported tokens: SOL, ETH, BTC, BNB, AVAX, DOGE (all paired with USDC)
- onchainos CLI installed and authenticated (`onchainos --version` and `onchainos wallet status`)
- Python 3.8+ (standard library only — no `pip install` required)
- Sufficient balance on your chosen chain for trading

## Quick Start

1. **Install the skill**: `plugin-store install mainstream-spot-order`
2. **Collect data**: Run `python3 collect.py --pair SOL/USDC` to pull 15-minute bars
3. **Run backtest**: Run `python3 backtest.py --pair SOL/USDC` to validate signal performance
4. **Start paper trading** (default, `PAPER_TRADE = True`): Run `python3 live.py`
5. **Open dashboard**: Visit `http://localhost:3250` to monitor signals and open positions
6. **Go live**: Set `PAPER_TRADE = False` in `config.py` and restart — confirm balance and risk parameters before switching
