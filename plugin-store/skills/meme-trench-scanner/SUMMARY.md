## Overview

Meme Trench Scanner is a Solana meme-token trading bot that scans 11 launchpads every 10 seconds, detects entries via TX acceleration, volume surge, and buy-sell ratio signals, runs deep safety checks, and manages exits through a 7-layer system.

Core operations:

- Scan 11 Solana launchpads (pump.fun, Believe, LetsBonk, and more) every 10 seconds for new token entries
- Detect signals via TX acceleration + volume surge + 5m/15m buy-sell ratio
- Run deep safety checks: dev rug history, bundler holdings, LP lock, aped wallets
- Manage exits via 7-layer system: emergency exit, FAST_DUMP crash detection, stop loss, trailing stop, tiered TP
- Observe all activity via TraderSoul AI and a live web dashboard

Tags: `meme-coin` `solana` `trading-bot` `launchpad` `onchainos` `pump.fun`

## Prerequisites

- No IP/region restrictions
- Supported chain: Solana
- Supported tokens: Newly launched Solana meme tokens across 11 launchpads
- onchainos CLI ≥ 2.1.0 installed and authenticated (`onchainos --version` and `onchainos wallet status`)
- Python 3.8+ (standard library only — no `pip install` required)
- Funded Solana wallet for live trading

## Quick Start

1. **Install the skill**: `plugin-store install meme-trench-scanner`
2. **Configure risk**: Edit `config.py` to pick Conservative / Default / Aggressive and tune `MAX_SOL`, `SOL_PER_TRADE`, `TP1_PCT`, `TP2_PCT`, `S1_PCT`, `MAX_POSITIONS`, `MAX_HOLD_MIN`
3. **Start in paper mode** (default, `PAPER_TRADE = True`): Run `python3 scan_live.py`
4. **Open dashboard**: Visit `http://localhost:3241` to monitor signals, positions, and TraderSoul observations
5. **Go live**: Set `PAUSED = False` to allow new positions, then `PAPER_TRADE = False` to use real funds — re-confirm exposure parameters before switching
6. **Stop anytime**: `pkill -f scan_live.py`
