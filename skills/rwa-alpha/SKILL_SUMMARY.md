# rwa-alpha — Skill Summary

## Overview
RWA Alpha is a Real World Asset intelligence trading skill that combines macro event detection with on-chain price action to auto-trade tokenized treasury, gold, yield, and governance tokens via OKX DEX. The perception layer polls NewsNow headlines (wallstreetcn, cls, jin10), Polymarket prediction markets, gold price feeds, and volume spike detection every 60 seconds. A 3-layer cognition pipeline classifies events: keyword regex for fast matching across 15 macro event types, LLM confirmation (Haiku) for ambiguous matches in the 0.55-0.80 confidence band, and LLM discovery for relevant headlines that miss all keywords. The macro playbook maps each event to target tokens, direction, and conviction. Execution goes through onchainos DEX quote/swap with Agentic Wallet TEE signing, guarded by position limits, daily trade caps, session stop-loss, cooldown timers, liquidity minimums, and portfolio-level max drawdown. Two exit systems: NAV premium/discount arbitrage for asset-backed tokens (USDY, OUSG, sDAI, bIB01, PAXG, XAUT, USDe) and TP/SL/trailing stop for governance tokens (ONDO, CFG, MPL, PENDLE, PLUME, OM, GFI, TRU).

## Usage
Start with `python3 rwa_alpha.py` — the skill begins polling news sources and on-chain data immediately. Configure strategy in `config.py`: set `MODE` (paper/live), `STRATEGY_MODE` (yield_optimizer/macro_trader/full_alpha), `TOTAL_BUDGET_USD`, and `ENABLED_CHAINS`. LLM classification requires `ANTHROPIC_API_KEY`. Dashboard auto-starts at `http://localhost:3249`. Prerequisites: onchainos CLI >= 2.1.0, Python >= 3.8, wallet login for live mode.

## Commands
| Command | Description |
|---|---|
| `python3 rwa_alpha.py` | Start the RWA trading engine + dashboard |
| `onchainos wallet login` | Authenticate wallet (required for live mode) |
| `onchainos wallet status` | Check wallet connection status |

## Triggers
Activates when the user mentions RWA, real world asset, tokenized treasury, gold token, USDY, OUSG, PAXG, ONDO, CFG, PENDLE, PLUME, OM, GFI, TRU, bIB01, yield rotation, macro trading, macro event, NAV premium, NAV discount, credit expansion, credit tightening.
