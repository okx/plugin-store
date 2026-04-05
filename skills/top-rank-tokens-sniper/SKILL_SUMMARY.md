# top-rank-tokens-sniper — Skill Summary

## Overview
Top Rank Tokens Sniper is a fully automated Solana trading bot that scans the OKX 1-hour gainers leaderboard every 10 seconds and snipes tokens the moment they first appear in the Top 20. Each candidate is scored on a 0–125 momentum scale (buy ratio, price change, active trader count, liquidity) and filtered through 25 safety checks across three levels: 13 Slot Guard pre-checks, 9 Advanced Safety checks, and 3 Holder Risk checks. The highest-priority exit rule is the ranking exit — 100% of the position is sold automatically the instant a token drops off the Top 20. Additional exits include a hard stop loss (-15%), quick stop (-8% within 3 minutes), trailing stop (activates at +10%, 8% drawdown trigger), time-based stop (2-hour max hold), and tiered take profit (TP1 +8% / TP2 +20% / TP3 +40%). All trades use the onchainos Agentic Wallet with TEE signing. Dashboard at `http://localhost:3244`.

## Usage
Run the AI startup protocol: the agent presents a risk questionnaire (Conservative / Default / Aggressive) that sets stop loss, TP tiers, and max hold duration in `config.py`, optionally switches to Live Mode with budget confirmation, then starts the bot with `python3 ranking_sniper.py`. Prerequisites: onchainos CLI >= 2.0.0-beta and `onchainos wallet login`.

## Commands
| Command | Description |
|---|---|
| `python3 ranking_sniper.py` | Start the main ranking scanner and sniper bot |
| `onchainos wallet login` | Authenticate the TEE agentic wallet |

## Triggers
Activates when the user mentions OKX ranking sniper, top-rank-tokens-sniper, leaderboard sniping, Solana 1h gainers bot, or onchainos ranking-based trading strategy.
