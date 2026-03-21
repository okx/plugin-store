---
name: memepump-scanner
description: "Use when the user asks about meme token scanning, pump.fun scanner, Trenches auto-scan, memepump safety filter, 扫链策略, 扫链v2, dev rug detection, bundler filtering, automated meme trading, scan migrated tokens, SOL meme scanner, pump.fun sniping strategy, or wants to run/configure/monitor the memepump scanner bot. Covers: safety-filtered scanning of MIGRATED pump.fun tokens, 3-signal momentum detection (TX acceleration + volume surge + buy pressure), cost-aware TP/SL, session risk controls. Do NOT use for manual token lookup — use okx-dex-trenches. Do NOT use for grid trading — use strategy-grid-trade."
license: Apache-2.0
metadata:
  author: Plugin Store
  category: "MEME Trading"
  chain: Solana
  version: "0.1.18"
  homepage: "https://github.com/okx/plugin-store"
---

# SOL Memepump Scanner v1.0.0

Automated safety-filtered meme token scanner + trader on Solana. Scans pump.fun MIGRATED tokens via OKX Trenches API, applies 22-point safety filter, detects 3-signal momentum, and executes trades with cost-aware exit management.

## Pre-flight Checks

Run immediately when this skill is triggered — before any response or command. Always follow these steps in order.

1. **Check onchainos**: `which onchainos` — if not found, tell user to install: https://web3.okx.com/zh-hans/onchainos/dev-docs/home/install-your-agentic-wallet; if found, run `onchainos --version` and verify version is **>= 2.0.0** — if older, tell user to upgrade
2. **Check binary**: `which strategy-memepump-scanner` — if not found, install via `plugin-store install memepump-scanner`
3. **Install onchainos-skills** (silent, non-blocking):
   ```bash
   npx skills add okx/onchainos-skills --yes 2>/dev/null || true
   ```
   This installs sub-skills used by Skill Routing (`okx-dex-token`, `okx-dex-swap`, etc.). Do not block on failure.
4. **Check wallet login**: `onchainos wallet status` — must show `loggedIn: true`; if not, run `onchainos wallet login`
5. **Check balance**: `strategy-memepump-scanner balance` — if `sufficient: false`, prompt user to top up

## Skill Routing

- For manual meme token lookup / dev check / bundle check → use `okx-dex-trenches`
- For token search / analytics → use `okx-dex-token`
- For DEX swap → use `okx-dex-swap`
- For token prices / charts → use `okx-dex-market`
- For wallet balances → use `okx-wallet-portfolio`
- For grid trading → use `strategy-grid-trade`
- For ranking-based sniping → use `ranking-sniper`
- For smart money signals → use `signal-tracker`

## Architecture

```
Trenches API → Server Filter (14) → Client Filter (3) → Signal Detection → Deep Safety → Buy → 8-Layer Exit → Sell
               (MC, Age, Holders    (B/S, Vol/MC,        (Sig A+B+C          (Dev rug=0,        (Emergency, SL,
                Vol, Bundler...)     Top10)               momentum)            Bundler ATH<25%)   TP1, BE, Trail, TP2, Time, MaxHold)
```

## Quickstart

```bash
# View current config
strategy-memepump-scanner config

# Analyze current market (top MIGRATED tokens)
strategy-memepump-scanner analyze

# Dry-run (no real trades)
strategy-memepump-scanner tick --dry-run

# Start live bot
strategy-memepump-scanner start

# Monitor
strategy-memepump-scanner status

# Emergency exit
strategy-memepump-scanner sell-all

# Stop bot
strategy-memepump-scanner stop
```

## Command Index

| # | Command | Auth | Description |
|---|---------|------|-------------|
| 1 | `strategy-memepump-scanner tick [--dry-run]` | Yes | Execute one scan cycle |
| 2 | `strategy-memepump-scanner start [--dry-run]` | Yes | Start continuous bot (tick every 10s) |
| 3 | `strategy-memepump-scanner stop` | No | Stop running bot |
| 4 | `strategy-memepump-scanner status` | No | Show positions, session stats, PnL |
| 5 | `strategy-memepump-scanner report` | No | Detailed PnL report |
| 6 | `strategy-memepump-scanner history [--limit N]` | No | Trade history |
| 7 | `strategy-memepump-scanner analyze` | No* | Show top MIGRATED tokens from Trenches |
| 8 | `strategy-memepump-scanner config` | No | Show all configurable parameters |
| 9 | `strategy-memepump-scanner sell-all` | Yes | Force-sell all open positions |
| 10 | `strategy-memepump-scanner sell <addr> --amount <raw>` | Yes | Sell specific token |
| 11 | `strategy-memepump-scanner test-trade <addr> [--amount]` | Yes | Buy+sell round-trip (debug) |
| 12 | `strategy-memepump-scanner reset --force` | No | Clear all state data |
| 13 | `strategy-memepump-scanner balance` | No | Check wallet balance sufficiency |

## Before Starting the Bot

**IMPORTANT:** Before `strategy-memepump-scanner start`:

1. Run `strategy-memepump-scanner config` — show current parameters
2. Present parameters in a readable table, ask user if they want to adjust
3. If adjusting, edit `~/.plugin-store/memepump_scanner_config.json` directly
4. Optionally do a `--dry-run` tick first to validate filters

## Operation Flow

### Intent: Start Scanning

```
1. strategy-memepump-scanner config          → review parameters
2. strategy-memepump-scanner analyze         → see current market
3. strategy-memepump-scanner tick --dry-run  → validate filters
4. strategy-memepump-scanner start           → go live
5. strategy-memepump-scanner status          → monitor
```

### Intent: Check and Emergency Exit

```
1. strategy-memepump-scanner status    → see positions + PnL
2. strategy-memepump-scanner sell-all  → emergency exit
3. strategy-memepump-scanner report    → review stats
```

### Intent: Research a Token

```
1. strategy-memepump-scanner status           → get token address
2. okx-dex-trenches (token detail)            → deep token info
3. okx-dex-market (price chart)               → chart
```

## Key Parameters (Defaults)

| Parameter | Default | Description |
|-----------|---------|-------------|
| `stage` | MIGRATED | Only scan pump.fun graduated tokens |
| `scalp_sol` | 0.0375 | SOL per SCALP tier buy |
| `minimum_sol` | 0.075 | SOL per MINIMUM tier buy |
| `max_positions` | 7 | Max simultaneous positions |
| `tp1_pct` | +15% | Take-profit tier 1 (+ breakeven offset) |
| `tp2_pct` | +25% | Take-profit tier 2 (+ breakeven offset) |
| `sl_scalp` | -15% | Stop-loss for SCALP tier |
| `sl_hot` | -20% | Stop-loss for HOT launch |
| `sl_quiet` | -25% | Stop-loss for QUIET launch |
| `max_hold_min` | 30 | Maximum hold time (minutes) |

> Full parameter reference, safety filter details, signal engine, and exit system: see `references/strategy-reference.md`

## Risk Controls

| Risk | Action |
|------|--------|
| Dev rug history (any) | BLOCK — zero tolerance |
| Dev farm (>20 launches) | BLOCK |
| Bundler ATH > 25% | BLOCK |
| 2 consecutive losses | Pause 15 min |
| 0.05 SOL cumulative loss | Pause 30 min |
| 0.10 SOL cumulative loss | Session terminated |
| 5 consecutive errors | Circuit breaker (1h cooldown) |
| Max positions reached | Skip new buys, continue exits |

## Response Guidelines

- Always show config before starting the bot
- Suggest `--dry-run` for first-time users
- After any action, suggest 2-3 natural follow-ups
- Support both English and Chinese — respond in the user's language
- Never expose raw transaction data or internal addresses
