---
name: signal-tracker
description: "Use when the user asks about smart money signal trading, 聪明钱策略, KOL following, whale tracking, signal bot, 信号策略, 跟单策略, 同车地址, cost-aware TP/SL, session risk controls, or wants to run/configure/monitor the signal tracker bot. Covers: OKX Signal API polling (SmartMoney/KOL/Whale), 17-point safety filter with Dev/Bundler checks, cost-aware take-profit with breakeven offset, time-decay stop-loss, trailing stop, session risk management (consecutive loss pause / cumulative loss halt). Do NOT use for meme token scanning — use memepump-scanner. Do NOT use for grid trading — use strategy-grid-trade. Do NOT use for manual signal lookup — use okx-dex-signal."
license: Apache-2.0
metadata:
  author: Plugin Store
  category: "MEME Trading"
  chain: Solana
  version: "0.1.18"
  homepage: "https://github.com/okx/plugin-store"
---

# SOL Signal Tracker v1.0.0

Automated smart-money signal following strategy on Solana. Polls OKX Signal API every 20s for SmartMoney/KOL/Whale buy signals, applies 17-point safety filter (Dev/Bundler zero-tolerance), executes cost-aware trades with multi-tier TP/SL, trailing stop, time-decay SL, and session risk controls.

## Pre-flight Checks

Run immediately when this skill is triggered — before any response or command. Always follow these steps in order.

1. **Check onchainos**: `which onchainos` — if not found, tell user to install: https://web3.okx.com/zh-hans/onchainos/dev-docs/home/install-your-agentic-wallet; if found, run `onchainos --version` and verify version is **>= 2.0.0** — if older, tell user to upgrade
2. **Check binary**: `which strategy-signal-tracker` — if not found, install via `plugin-store install signal-tracker`
3. **Install onchainos-skills** (silent, non-blocking):
   ```bash
   npx skills add okx/onchainos-skills --yes 2>/dev/null || true
   ```
   This installs sub-skills used by Skill Routing (`okx-dex-token`, `okx-dex-swap`, etc.). Do not block on failure.
4. **Check wallet login**: `onchainos wallet status` — must show `loggedIn: true`; if not, run `onchainos wallet login`
5. **Check balance**: `strategy-signal-tracker balance` — if `sufficient: false`, prompt user to top up

## Skill Routing

- For manual signal lookup / what smart money is buying → use `okx-dex-signal`
- For meme token scanning (pump.fun) → use `memepump-scanner`
- For ranking-based sniping → use `ranking-sniper`
- For token search / analytics → use `okx-dex-token`
- For DEX swap → use `okx-dex-swap`
- For token prices / charts → use `okx-dex-market`
- For wallet balances → use `okx-wallet-portfolio`
- For grid trading → use `strategy-grid-trade`
- For dev/bundler manual check → use `okx-dex-trenches`

## Architecture

```
OKX Signal API → Pre-filter (4) → Deep Verify (13) → Buy → 7-Layer Exit → Sell
(SmartMoney/      (MC, Liq,        (Safety + Dev +      (Tier-sized:   (RUG_LIQ, Dust,
 KOL/Whale,        Wallets,          Bundler + k1pump     0.010–0.020    TIME_DECAY_SL,
 every 20s)        SoldRatio)        + Honeypot)          SOL)           HARD_SL, TP1/2/3,
                                                                         TRAILING, TIME_STOP)
```

## Quickstart

```bash
# View current config
strategy-signal-tracker config

# Dry-run (no real trades)
strategy-signal-tracker tick --dry-run

# Start live bot
strategy-signal-tracker start

# Start in dry-run mode
strategy-signal-tracker start --dry-run

# Monitor
strategy-signal-tracker status

# Emergency exit
strategy-signal-tracker sell-all

# Stop bot
strategy-signal-tracker stop
```

## Command Index

| # | Command | Auth | Description |
|---|---------|------|-------------|
| 1 | `strategy-signal-tracker tick [--dry-run]` | Yes | Execute one tick cycle |
| 2 | `strategy-signal-tracker start [--dry-run]` | Yes | Start continuous bot (tick every 20s) |
| 3 | `strategy-signal-tracker stop` | No | Stop running bot |
| 4 | `strategy-signal-tracker status` | No | Show positions, session stats, PnL |
| 5 | `strategy-signal-tracker report` | No | Detailed PnL report |
| 6 | `strategy-signal-tracker history [--limit N]` | No | Trade history |
| 7 | `strategy-signal-tracker config` | No | Show all configurable parameters |
| 8 | `strategy-signal-tracker sell-all` | Yes | Force-sell all open positions |
| 9 | `strategy-signal-tracker sell <addr> --amount <raw>` | Yes | Sell specific token |
| 10 | `strategy-signal-tracker test-trade <addr> [--amount]` | Yes | Buy+sell round-trip (debug) |
| 11 | `strategy-signal-tracker reset --force` | No | Clear all state data |
| 12 | `strategy-signal-tracker balance` | No | Check wallet balance sufficiency |

## Before Starting the Bot

**IMPORTANT:** Before `strategy-signal-tracker start`:

1. Run `strategy-signal-tracker config` — show current parameters
2. Present parameters in a readable table, ask user if they want to adjust
3. If adjusting, edit `~/.plugin-store/signal_tracker_config.json` directly
4. Optionally do a `--dry-run` tick first to validate filters

## Operation Flow

### Intent: Start Tracking

```
1. strategy-signal-tracker config           → review parameters
2. strategy-signal-tracker tick --dry-run   → validate filters
3. strategy-signal-tracker start            → go live
4. strategy-signal-tracker status           → monitor
```

### Intent: Check and Emergency Exit

```
1. strategy-signal-tracker status    → see positions + PnL
2. strategy-signal-tracker sell-all  → emergency exit
3. strategy-signal-tracker report    → review stats
```

### Intent: Research a Signal Token

```
1. strategy-signal-tracker status         → get token address + label
2. okx-dex-signal (manual lookup)         → current signal details
3. okx-dex-market (price chart)           → chart
```

## Key Parameters (Defaults)

| Parameter | Default | Description |
|-----------|---------|-------------|
| `signal_labels` | "1,2,3" | 1=SmartMoney, 2=KOL, 3=Whale |
| `min_wallet_count` | 3 | Minimum co-buying wallets |
| `max_sell_ratio` | 0.80 | Skip if smart money sold >80% |
| `min_mcap` | $200K | Minimum market cap |
| `min_liquidity` | $80K | Minimum liquidity |
| `position_high_sol` | 0.020 | SOL per trade (≥8 wallets) |
| `position_mid_sol` | 0.015 | SOL per trade (≥5 wallets) |
| `position_low_sol` | 0.010 | SOL per trade (≥3 wallets) |
| `max_positions` | 6 | Max simultaneous positions |
| `sl_multiplier` | 0.90 | Hard stop-loss (−10%) |
| `time_stop_hours` | 4.0 | Hard time stop |

> Full parameter reference, safety filter details, exit system, and CLI fields: see `references/strategy-reference.md`

## Risk Controls

| Risk | Action |
|------|--------|
| Dev rug history (any) | BLOCK — zero tolerance |
| 1m pump > 15% at entry | BLOCK — avoid chasing tops |
| Honeypot detected | BLOCK |
| Liquidity < $5K | Emergency exit (RUG_LIQ) |
| 3 consecutive losses | Pause 10 min |
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
