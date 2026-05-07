## Overview

Smart Money Signal Copy Trade is a Solana copy-trading bot that polls OKX Smart Money, KOL, and Whale buy signals every 20 seconds, triggers entries on co-rider consensus (≥3 wallets buying the same token simultaneously), and manages exits via a 7-layer system with cost-aware take profit.

Core operations:

- Poll OKX Smart Money / KOL / Whale buy signals every 20 seconds
- Trigger entry only on co-rider consensus: ≥3 tracked wallets buying the same token simultaneously
- Run 15 pre-trade safety filters: market cap, liquidity, holders, dev rug, bundler, LP burn, K1 pump, and more
- Manage exits via 7-layer system: cost-aware TP1/TP2/TP3, hard stop, time-decay SL, trailing stop, trend stop, liquidity emergency
- Hot-reload config changes without restarting the bot

Tags: `copy-trade` `smart-money` `kol` `whale` `solana` `onchainos` `signals`

## Prerequisites

- No IP/region restrictions
- Supported chain: Solana
- Supported tokens: Solana meme and trending tokens tracked by OKX Smart Money signals
- onchainos CLI ≥ 2.0.0 installed and authenticated (`onchainos --version` and `onchainos wallet status`)
- Python 3.8+ (standard library only — no `pip install` required)
- Funded Solana wallet for live trading

## Quick Start

1. **Install the skill**: `plugin-store install smart-money-signal-copy-trade`
2. **Configure risk**: Edit `config.py` to pick Conservative / Default / Aggressive and tune `POSITION_TIERS`, `MAX_POSITIONS`, `MIN_WALLET_COUNT`, `TP_TIERS`, `SL_MULTIPLIER`, `TRAIL_ACTIVATE`
3. **Start in paper mode** (default, `DRY_RUN = True`): Run `python3 bot.py`
4. **Open dashboard**: Visit `http://localhost:3248` to monitor signals, positions, and smart money activity
5. **Go live**: Set `PAUSED = False` to allow new positions, then `DRY_RUN = False` to use real funds — re-confirm budget and loss limits before switching
6. **Stop anytime**: `pkill -f bot.py`
