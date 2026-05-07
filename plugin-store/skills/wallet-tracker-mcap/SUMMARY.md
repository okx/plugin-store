## Overview

Wallet Tracker is a Solana copy-trading bot that monitors target wallets for meme token trades and automatically mirrors buy and sell actions with MC target gating, 4-tier risk grading, and a 5-trigger exit system.

Core operations:

- Monitor target Solana wallets for meme token buy/sell activity in real time
- Mirror trades using MC_TARGET mode (wait for market cap proof) or INSTANT mode (immediate follow)
- Run pre-trade safety gates: liquidity, holders, top10, dev hold, bundle, honeypot, rug history checks
- Manage exits via 5 triggers: mirror sell, stop loss, tiered take-profit, trailing stop, time stop
- Monitor positions post-trade for active dump, LP drain, and coordinated selling → auto exit

Tags: `wallet-tracker` `copy-trade` `solana` `meme-coin` `onchainos`

## Prerequisites

- No IP/region restrictions
- Supported chain: Solana
- Supported tokens: Solana meme tokens (pump.fun, Believe, LetsBonk, and other launchpads)
- onchainos CLI installed and authenticated (`onchainos --version` and `onchainos wallet status`)
- Python 3.8+ (standard library only — no `pip install` required)
- Funded Solana wallet for live trading
- At least one target wallet address to track

## Quick Start

1. **Install the skill**: `plugin-store install wallet-tracker-mcap`
2. **Add target wallets**: Edit `config.py` and add wallet addresses to the `WATCH_WALLETS` list
3. **Choose follow mode**: Set `FOLLOW_MODE = "MC_TARGET"` (safer, waits for MC confirmation) or `"INSTANT"` (faster, follows immediately)
4. **Start in paper mode** (default, `PAPER_TRADE = True`): Run `python3 bot.py`
5. **Open dashboard**: Visit `http://localhost:3248` to monitor watched wallets, positions, and live trade feed
6. **Go live**: Set `PAPER_TRADE = False` in `config.py` and restart — confirm `MAX_SOL` and risk limits before switching
