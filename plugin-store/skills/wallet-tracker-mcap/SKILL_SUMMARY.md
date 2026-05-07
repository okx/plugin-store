# wallet-tracker-mcap -- Skill Summary

## Overview
Wallet Tracker (Mcap) is a real-time wallet copy-trading bot for Solana meme tokens. It monitors target wallets via `onchainos portfolio all-balances`, detects new token acquisitions through holding snapshots, validates each token with safety filters and 4-tier risk grading (honeypot, rug, wash trade, LP drain), then follows via MC_TARGET mode (waits for market cap proof) or INSTANT mode. Exits via mirror sell, stop loss (-20%), tiered take-profit (+15%/+30%/+50%), trailing stop, time stop (6h), or risk alert (active dump/LP drain). Paper Mode (MODE="paper") + PAUSED=True is the default. Web dashboard at `http://localhost:3248` shows positions, watch list, trade history, and live feed.

## Usage
Run the AI startup protocol: the agent explains the strategy, asks for target wallet address(es), asks paper/live mode, asks follow mode (MC_TARGET default), asks risk profile (conservative/default/aggressive), validates onchainos CLI login, then launches via `python3 wallet_tracker.py`. Prerequisites: onchainos CLI >= 2.1.0, `onchainos wallet login`, Python 3.8+ (no pip install needed).

## Commands
| Command | Description |
|---|---|
| `python3 wallet_tracker.py` | Start the bot + dashboard on port 3248 |
| `POST /api/pause` | Toggle trading pause via dashboard |
| `POST /api/set-mc-target` | Update MC target at runtime |
| `POST /api/reset-snapshot` | Re-baseline wallet holdings |
| `onchainos wallet login` | Authenticate the TEE agentic wallet |

## Triggers
Activates when the user mentions wallet tracker, copy trade, follow wallet, mirror trade, wallet monitor, 跟单, 钱包跟踪, 钱包监控, 抄单, 跟买跟卖, wallet sniper, smart money follow, whale tracker, or mcap target.
