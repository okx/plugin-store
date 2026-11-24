## Overview

Top Rank Tokens Sniper is a Solana ranking-leaderboard sniper that scans the OKX 1-hour gainers Top 20 every 10 seconds, snipes tokens on their first leaderboard appearance after 25 safety checks, and auto-exits when a token drops out of the Top 20.

Core operations:

- Scan OKX 1-hour gainers Top 20 leaderboard every 10 seconds for new entries
- Score candidates with a 0–125 momentum score (buy ratio, price change, active traders, liquidity)
- Run 25 pre-trade checks: 13 Slot Guard + 9 Advanced Safety + 3 Holder Risk checks
- Manage exits via 6-layer system: rank-out (auto-sell when token leaves Top 20), stop loss, trailing stop, tiered TP, time stop, emergency exit
- Monitor all positions and leaderboard activity on a live web dashboard

Tags: `sniper` `leaderboard` `solana` `meme-coin` `onchainos` `momentum`

## Prerequisites

- No IP/region restrictions
- Supported chain: Solana
- Supported tokens: OKX 1-hour gainers Top 20 leaderboard tokens on Solana
- onchainos CLI ≥ 2.0.0 installed and authenticated (`onchainos --version` and `onchainos wallet status`)
- Python 3.8+ (standard library only — no `pip install` required)
- Funded Solana wallet for live trading

## Quick Start

1. **Install the skill**: `plugin-store install top-rank-tokens-sniper`
2. **Configure risk**: Edit `config.py` to pick Conservative / Default / Aggressive and tune `BUY_AMOUNT`, `TOTAL_BUDGET`, `MAX_POSITIONS`, `TP_TIERS`, `STOP_LOSS_PCT`, `TRAILING_ACTIVATE`, `MAX_HOLD_HOURS`
3. **Start in paper mode** (default, `MODE = "paper"`): Run `python3 ranking_sniper.py`
4. **Open dashboard**: Visit `http://localhost:3244` to monitor the leaderboard, positions, and momentum scores
5. **Go live**: Set `PAUSED = False` to allow new positions, then `MODE = "live"` to use real funds — re-confirm budget and per-trade size before switching
6. **Stop anytime**: `pkill -f ranking_sniper.py`
