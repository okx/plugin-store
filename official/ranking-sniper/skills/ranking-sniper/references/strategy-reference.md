# Ranking Sniper — Strategy & CLI Reference

Complete reference for the 3-layer safety filter, momentum scoring, 6-layer exit system, all CLI commands, and configurable parameters.

---

## 3-Layer Safety Filter (25 Checks)

### Layer 1: Slot Guard (13 checks from ranking data)

| # | Check | Default | Production |
|---|-------|---------|------------|
| 1 | Price change min | >= 1% | 15% |
| 2 | Price change max | <= 150% | 150% |
| 3 | Liquidity | >= $1,000 | $5,000 |
| 4 | Market cap min | >= $1,000 | $5,000 |
| 5 | Market cap max | <= $50M | $10M |
| 6 | Holders | >= 5 | 30 |
| 7 | Buy ratio (buy/total TX) | >= 40% | 55% |
| 8 | Unique traders | >= 5 | 20 |
| 9 | Skip system tokens | SOL, USDC, etc. | — |
| 10 | Cooldown check | Not in cooldown | — |
| 11 | Position limit | < max_positions | — |
| 12 | Already holding | Not holding | — |
| 13 | Daily loss limit | Not exceeded | — |

### Layer 2: Advanced Safety (9 checks from advanced-info API)

| # | Check | Default | Production |
|---|-------|---------|------------|
| 14 | Risk control level | <= 3 | 1 |
| 15 | Honeypot tag | Not present | — |
| 16 | Top 10 concentration | <= 80% | 50% |
| 17 | Dev holding | <= 50% | 20% |
| 18 | Bundler holding | <= 50% | 15% |
| 19 | LP burned | >= 0% | 80% |
| 20 | Dev rug pull count | <= 100 | 10 |
| 21 | Sniper holding | <= 50% | 20% |
| 22 | Block internal (PumpFun) | false | true |

### Layer 3: Holder Risk Scan (3 checks from holder API)

| # | Check | Default | Production |
|---|-------|---------|------------|
| 23 | Suspicious holder total % | <= 50% | 10% |
| 24 | Suspicious holder count | <= 50 | 5 |
| 25 | Phishing holders | allowed | blocked |

> Default thresholds are relaxed for testing. For production, update config with "Production" values.

---

## Momentum Score (0-125)

### Base Score (0-100)

| Component | Max | Formula |
|-----------|-----|---------|
| Buy Score | 40 | buy_ratio × 40 |
| Change Penalty | 20 | if change > 100%: 20 - (change-100)/10; else: change/5 |
| Trader Score | 20 | min(traders/50, 1) × 20 |
| Liquidity Score | 20 | min(liquidity/50000, 1) × 20 |

### Bonus Score (0-25, capped)

| Bonus | Points | Condition |
|-------|--------|-----------|
| Smart Money | +8 | `smartMoneyBuy` tag present |
| Low Concentration | +5/+2 | Top10 < 30% / < 50% |
| DS Paid | +3 | `dsPaid` tag present |
| Community | +2 | `dexScreenerTokenCommunityTakeOver` tag |
| Low Sniper | +4/+2 | Sniper < 5% / < 10% |
| Dev Clean | +3 | Dev hold 0% AND rug count < 3 |
| Zero Suspicious | +2 | No active suspicious holders |

**Buy threshold:** Default 10 (testing), production 40.

---

## 6-Layer Exit System

Priority order (first match exits):

| Layer | Exit Type | Condition | Action |
|-------|-----------|-----------|--------|
| 1 | Ranking Exit | Token drops off top N (after 60s) | FULL sell |
| 2 | Hard Stop | PnL <= -25% | FULL sell |
| 3 | Fast Stop | PnL <= -8% after 5 minutes | FULL sell |
| 4 | Trailing Stop | Drawdown >= 12% from peak (activates at +8%) | FULL sell |
| 5 | Time Stop | Elapsed >= 6h | FULL sell |
| 6 | Gradient TP | PnL >= TP level | PARTIAL sell |

### Gradient Take-Profit Levels

| Level | Trigger | Sell Portion |
|-------|---------|-------------|
| TP1 | +5% | 25% |
| TP2 | +15% | 35% |
| TP3 | +30% | 40% |

---

## Configurable Parameters

Config file: `~/.plugin-store/ranking_sniper_config.json`

### Money Management

| Parameter | Default | Description |
|-----------|---------|-------------|
| `budget_sol` | 0.5 | Total SOL budget |
| `per_trade_sol` | 0.05 | SOL per buy trade |
| `max_positions` | 5 | Maximum simultaneous positions |
| `gas_reserve_sol` | 0.01 | SOL reserved for gas |
| `min_wallet_balance` | 0.1 | Minimum wallet balance to maintain |
| `daily_loss_limit_pct` | 15.0 | Daily loss limit (% of budget) |
| `dry_run` | false | Simulate without executing swaps |

### Trading Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `slippage_pct` | "3" | DEX slippage tolerance (%) |
| `score_buy_threshold` | 10 | Momentum score threshold (0-125) |
| `tick_interval_secs` | 10 | Polling interval (seconds) |
| `cooldown_minutes` | 30 | Post-sell cooldown per token |
| `top_n` | 20 | Ranking entries to scan |

### Exit System

| Parameter | Default | Description |
|-----------|---------|-------------|
| `hard_stop_pct` | -25.0 | Hard stop-loss (%) |
| `fast_stop_time_secs` | 300 | Fast stop window (seconds) |
| `fast_stop_pct` | -8.0 | Fast stop threshold (%) |
| `trailing_activate_pct` | 8.0 | Trailing stop activation (%) |
| `trailing_drawdown_pct` | 12.0 | Trailing stop drawdown (%) |
| `time_stop_secs` | 21600 | Time stop (6h) |
| `tp_levels` | [5, 15, 30] | Gradient take-profit levels (%) |

### Circuit Breaker

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_consecutive_errors` | 5 | Errors before breaker trips |
| `cooldown_after_errors` | 3600 | Cooldown after breaker (seconds) |

---

## CLI Command Details

### strategy-ranking-sniper tick

Execute one tick: fetch ranking, check exits, scan new entries, execute trades.

```bash
strategy-ranking-sniper tick [--budget <sol>] [--per-trade <sol>] [--dry-run]
```

**Return fields:**

| Field | Description |
|-------|-------------|
| `tick_time` | ISO 8601 timestamp |
| `positions` | Number of open positions |
| `remaining_budget_sol` | Remaining SOL budget |
| `daily_pnl_sol` | Daily PnL in SOL |
| `actions` | Array of actions (buy/exit/skip/buy_failed) |
| `dry_run` | Whether this was a dry-run |

**Action types:**
- `buy` — position opened (symbol, price, amount_sol, score, tx_hash)
- `exit` — position closed (symbol, reason, exit_type, pnl_pct, pnl_sol, tx_hash)
- `skip` — token rejected (symbol, reason)
- `buy_failed` — swap failed (symbol, error)
- `exit_failed` — sell failed (symbol, reason, error)

### strategy-ranking-sniper start

Start foreground bot loop (tick every 10s). PID file: `~/.plugin-store/ranking_sniper.pid`. Log: `~/.plugin-store/ranking_sniper.log`.

```bash
strategy-ranking-sniper start [--budget <sol>] [--per-trade <sol>] [--dry-run]
```

### strategy-ranking-sniper status

**Return fields:**

| Field | Description |
|-------|-------------|
| `bot_running` | Whether bot is active |
| `stopped` / `stop_reason` | If stopped by limit |
| `positions` | Open position details |
| `remaining_budget_sol` | Remaining budget |
| `daily_pnl_sol` | Daily PnL |
| `consecutive_errors` | Error count |

### strategy-ranking-sniper report

**Return fields:**

| Field | Description |
|-------|-------------|
| `total_buys` / `total_sells` | Trade counts |
| `total_invested_sol` / `total_returned_sol` | SOL flows |
| `total_pnl_sol` / `daily_pnl_sol` | PnL |
| `win_count` / `loss_count` / `win_rate` | Win/loss stats |

### strategy-ranking-sniper sell-all

Force-sell all positions. Retries with halved amounts if liquidity is insufficient (up to 4 attempts).

**Return fields:** `sold`, `failed`, `results` (per-position: symbol, token, status, tx_hash, sol_out, error)

### strategy-ranking-sniper test-trade

```bash
strategy-ranking-sniper test-trade <token_address> [--amount <sol>]
```

**Return fields:** `buy.tx_hash`, `buy.price`, `buy.amount_out`, `sell.tx_hash`, `sell.amount_out`, `price_before`, `price_after`

---

## Execution Pipeline

```
fetch_ranking(top_n=20)              ← OKX /token/toplist (sort by 5m change)
    │
    ├─ For each existing position:
    │    fetch_price(token)          ← OKX /price-info
    │    check_exits(6 layers)       ← engine.rs pure function
    │    If exit signal → sell       ← OKX DEX swap + sign + broadcast
    │
    └─ For each new token in ranking:
         known_tokens check          ← skip if already seen
         budget + position check     ← skip if insufficient
         fetch_advanced_info()       ← OKX /token/advanced-info
         run_slot_guard(13 checks)
         run_advanced_safety(9 checks)
         fetch_holder_risk()         ← OKX /token/holder (tag 6 + 8)
         run_holder_risk_scan(3 checks)
         calc_momentum_score()       ← 0-125
         If score >= threshold:
           buy_token()               ← OKX DEX swap + sign + broadcast
```

---

## State Files

| File | Purpose |
|------|---------|
| `~/.plugin-store/ranking_sniper_state.json` | Full bot state (positions, trades, stats, known tokens) |
| `~/.plugin-store/ranking_sniper_config.json` | User-configurable parameters |
| `~/.plugin-store/ranking_sniper.pid` | PID file for running bot |
| `~/.plugin-store/ranking_sniper.log` | Execution log |

---

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| No ranking data | Saves state, outputs `no_ranking_data` |
| Circuit breaker (5 errors) | Rejects all ticks for 1h cooldown |
| Daily loss limit exceeded | Bot stops, requires `reset --force` |
| Sell fails (low liquidity) | `sell-all` retries with halved amounts (up to 4x) |
| Advanced-info API fails | Token skipped |
| Price fetch fails | Exit check skipped for that token |
| onchainos wallet not available | Error — please login first |
| Bot already running | `start` rejects with PID warning |

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| "onchainos wallet not available" | Not logged in | `onchainos wallet status` → login |
| Circuit breaker trips | Repeated failures | Check log, wait 1h or `reset --force` |
| No buys happening | Score/filters too strict | `--dry-run` to see skip reasons, adjust config |
| Sell fails repeatedly | Low liquidity | `sell-all` auto-retries, or manual `sell` |
| "Bot stopped" on tick | Daily loss or prior stop | `reset --force` |
| High slippage | Tolerance too low | Raise `slippage_pct` to 5-10% for memes |
