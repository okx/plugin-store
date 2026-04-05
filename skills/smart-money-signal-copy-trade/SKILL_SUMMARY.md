# smart-money-signal-copy-trade — Skill Summary

## Overview
Smart Money Signal Copy Trade is a fully automated Solana copy-trade bot that monitors OKX Smart Money, KOL, and Whale wallet activity every 20 seconds. It enters a position when a co-rider consensus is detected — three or more tracked smart wallets buying the same token simultaneously. Before entry, 15 safety checks are applied covering market cap, liquidity depth, holder distribution, dev rug history, bundler exposure, LP burn status, and K1 pump indicators. Take profit targets are cost-aware (TP1 +5% / TP2 +15% / TP3 +30% NET, accounting for fees and slippage), and positions are protected by a 7-layer exit system. A cross-strategy collision guard prevents overlap with other running bots. Config supports hot-reload without restarting. All trades execute via the onchainos Agentic Wallet with TEE signing. Dashboard at `http://localhost:3248`.

## Usage
Run the AI startup protocol: the agent presents a risk questionnaire (Conservative / Default / Aggressive) setting SL multiplier, max positions, and minimum wallet count in `config.py`, optionally switches to Live Mode with budget confirmation, then starts the bot with `python3 bot.py`. Prerequisites: onchainos CLI >= 2.0.0-beta and `onchainos wallet login`.

## Commands
| Command | Description |
|---|---|
| `python3 bot.py` | Start the main signal tracking and copy-trade bot |
| `onchainos wallet login` | Authenticate the TEE agentic wallet |

## Triggers
Activates when the user mentions smart money copy trade, KOL wallet signals, whale copy trading, smart-money-signal-copy-trade, or onchainos signal-based strategy on Solana.
