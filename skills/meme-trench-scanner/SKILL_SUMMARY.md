# meme-trench-scanner — Skill Summary

## Overview
Meme Trench Scanner is a fully automated Solana meme token trading bot that monitors 11 launchpads for new token launches and enters positions when triple signals align: TX acceleration, volume surge, and a favorable buy/sell ratio computed from 5-minute and 15-minute raw trade windows. It applies deep safety checks (dev rug history, bundler holdings, LP lock status, aped wallet detection) before entry, and manages each position through a 7-layer exit system including FAST_DUMP 10-second crash detection, TOP_ZONE ATH proximity filtering, tiered take profit (TP1 +15% / TP2 +25%), and a trailing stop. All wallet operations use the onchainos Agentic Wallet with TEE signing — no private key exposure or API key required. A web dashboard is served at `http://localhost:3241`.

## Usage
Run the AI startup protocol first: the agent presents a risk questionnaire (Conservative / Default / Aggressive) that sets TP/SL parameters in `config.py`, confirms whether to switch from Paper Mode to Live Trading, then launches the bot with `python3 scan_live.py`. Prerequisites: onchainos CLI >= 2.1.0 and `onchainos wallet login`. No pip dependencies — Python standard library only.

## Commands
| Command | Description |
|---|---|
| `python3 scan_live.py` | Start the main scanning and trading bot |
| `onchainos wallet login` | Authenticate the TEE agentic wallet |

## Triggers
Activates when the user mentions meme token scanning, Solana meme bot, meme-trench-scanner, pump.fun automation, launchpad sniping, or onchainos meme trading strategy.
