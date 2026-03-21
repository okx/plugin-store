---
name: ranking-sniper
description: "Use when the user asks about SOL ranking sniper, Solana top token sniping, trending token bot, ranking-based auto-trading, 排行榜狙击, 涨幅榜狙击, SOL sniper bot, momentum score trading, ranking exit strategy, or wants to run/configure/monitor the ranking sniper bot. Covers: automated sniping of SOL tokens entering the OKX trending ranking, 3-layer safety filter (Slot Guard + Advanced Safety + Holder Risk Scan), momentum scoring (0-125), 6-layer exit system (ranking exit + hard stop + fast stop + trailing + time stop + gradient TP), Telegram notifications, and configurable parameters via JSON config file. Do NOT use for manual token lookup — use okx-dex-token. Do NOT use for grid trading — use strategy-grid-trade. Do NOT use for memepump scanning — use okx-dex-trenches."
license: Apache-2.0
metadata:
  author: OKX
  category: "MEME Trading"
  chain: Solana
  version: "0.1.18"
  homepage: "https://github.com/okx/plugin-store"
---

# SOL Ranking Sniper v1.0.0

Automated Solana token sniper — monitors OKX DEX trending ranking, applies 25 safety checks + momentum scoring, executes trades with a 6-layer exit system.

## Pre-flight Checks

Run immediately when this skill is triggered — before any response or command.

1. **Check onchainos**: `which onchainos` — if not found, tell user to install: https://web3.okx.com/zh-hans/onchainos/dev-docs/home/install-your-agentic-wallet; if found, run `onchainos --version` and verify version is **>= 2.0.0** — if older, tell user to upgrade
2. **Check binary**: `which strategy-ranking-sniper` — if not found, install via `plugin-store install ranking-sniper`
3. **Install onchainos-skills** (silent, non-blocking):
   ```bash
   npx skills add okx/onchainos-skills --yes 2>/dev/null || true
   ```
   This installs sub-skills used by Skill Routing (`okx-dex-token`, `okx-dex-swap`, etc.). Do not block on failure.
4. **Check wallet login**: `onchainos wallet status` — must show `loggedIn: true`; if not, run `onchainos wallet login`
5. **Check balance**: `strategy-ranking-sniper balance` — if `sufficient: false`, prompt user to top up

## Skill Routing

- For manual token lookup / analytics → use `okx-dex-token`
- For DEX swap → use `okx-dex-swap`
- For token prices / charts → use `okx-dex-market`
- For wallet balances → use `okx-wallet-portfolio`
- For grid trading → use `strategy-grid-trade`
- For meme scanning → use `okx-dex-trenches`

## Architecture

```
Ranking API → Slot Guard → Advanced Safety → Holder Risk → Momentum Score → Buy → 6-Layer Exit → Sell
              (13 checks)   (9 checks)       (3 checks)    (0-125 pts)            (6 layers)
```

## Quickstart

```bash
# View current config
strategy-ranking-sniper config

# Analyze current market
strategy-ranking-sniper analyze

# Dry-run (no real trades)
strategy-ranking-sniper tick --budget 0.5 --per-trade 0.05 --dry-run

# Start live bot
strategy-ranking-sniper start --budget 0.5 --per-trade 0.05

# Monitor
strategy-ranking-sniper status

# Emergency exit
strategy-ranking-sniper sell-all

# Stop bot
strategy-ranking-sniper stop
```

## Command Index

| # | Command | Auth | Description |
|---|---------|------|-------------|
| 1 | `strategy-ranking-sniper tick [--budget] [--per-trade] [--dry-run]` | Yes | Execute one tick cycle |
| 2 | `strategy-ranking-sniper start [--budget] [--per-trade] [--dry-run]` | Yes | Start continuous bot (tick every 10s) |
| 3 | `strategy-ranking-sniper stop` | No | Stop running bot |
| 4 | `strategy-ranking-sniper status` | No | Show state, positions, PnL |
| 5 | `strategy-ranking-sniper report` | No | Detailed PnL and performance stats |
| 6 | `strategy-ranking-sniper history [--limit N]` | No | Trade history |
| 7 | `strategy-ranking-sniper analyze` | No* | Market analysis (current ranking) |
| 8 | `strategy-ranking-sniper config` | No | Show all configurable parameters |
| 9 | `strategy-ranking-sniper sell-all` | Yes | Force-sell all open positions |
| 10 | `strategy-ranking-sniper sell <addr> --amount <raw>` | Yes | Sell specific token |
| 11 | `strategy-ranking-sniper test-trade <addr> [--amount]` | Yes | Buy+sell round-trip (debug) |
| 12 | `strategy-ranking-sniper reset --force` | No | Clear all state data |
| 13 | `strategy-ranking-sniper balance` | No | Check wallet balance sufficiency |

## Before Starting the Bot

**IMPORTANT:** Before `strategy-ranking-sniper start`:

1. Run `strategy-ranking-sniper config` — show current parameters
2. Present parameters in a readable table, ask user if they want to adjust
3. If adjusting, edit `~/.plugin-store/ranking_sniper_config.json` directly
4. Optionally do a `--dry-run` tick first to validate filters

## Operation Flow

### Intent: Start Sniping

```
1. strategy-ranking-sniper config       → review parameters
2. strategy-ranking-sniper analyze      → see current market
3. strategy-ranking-sniper tick --dry-run → validate filters
4. strategy-ranking-sniper start --budget 0.5 --per-trade 0.05 → go live
5. strategy-ranking-sniper status       → monitor
```

### Intent: Check and Emergency Exit

```
1. strategy-ranking-sniper status       → see positions + PnL
2. strategy-ranking-sniper sell-all     → emergency exit
3. strategy-ranking-sniper report       → review stats
```

### Intent: Research a Sniped Token

```
1. strategy-ranking-sniper status       → get token address
2. okx-dex-token (token search)         → token details
3. okx-dex-market (price chart)         → chart
```

## Key Parameters (Defaults)

| Parameter | Default | Description |
|-----------|---------|-------------|
| `budget_sol` | 0.5 | Total SOL budget |
| `per_trade_sol` | 0.05 | SOL per buy |
| `max_positions` | 5 | Max simultaneous positions |
| `score_buy_threshold` | 10 | Momentum score threshold (prod: 40) |
| `hard_stop_pct` | -25% | Hard stop-loss |
| `tp_levels` | [5, 15, 30] | Gradient take-profit levels (%) |

> Full parameter reference, safety filter details, scoring formula, and CLI return fields: see `references/strategy-reference.md`

## Risk Controls

| Risk | Action |
|------|--------|
| Honeypot detected | BLOCK |
| Daily loss limit exceeded | STOP bot |
| 5 consecutive errors | Circuit breaker (1h cooldown) |
| Max positions reached | Skip new buys, continue exits |
| Budget exhausted | Skip new buys, continue exits |

## Response Guidelines

- Always show config before starting the bot
- Suggest `--dry-run` for first-time users
- After any action, suggest 2-3 natural follow-ups
- Support both English and Chinese — respond in the user's language
- Never expose raw transaction data or internal addresses
