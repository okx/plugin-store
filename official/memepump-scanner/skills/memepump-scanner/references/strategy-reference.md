# Memepump Scanner — Strategy & CLI Reference

Complete reference for the 22-point safety filter, 3-signal detection engine, cost model, 8-layer exit system, all CLI commands, and configurable parameters.

---

## 22-Point Safety Filter

### Layer 1: Server-Side Filter (14 checks, API-enforced, zero extra cost)

| # | Filter | Default | Description |
|---|--------|---------|-------------|
| 1 | Market Cap Min | ≥ $80K | Prevent low-MC rug traps |
| 2 | Market Cap Max | ≤ $800K | Meme sweet spot |
| 3 | Holders | ≥ 50 | Minimum distribution |
| 4 | Dev Holdings | ≤ 10% | Prevent dev dump |
| 5 | Bundler % | ≤ 15% | Prevent bot manipulation |
| 6 | Sniper % | ≤ 20% | Prevent sniper sell pressure |
| 7 | Insider % | ≤ 15% | Prevent insider trading |
| 8 | Top10 Holdings | ≤ 50% | Prevent whale control |
| 9 | Fresh Wallets | ≤ 40% | Prevent wash trading |
| 10 | Total TX | ≥ 30 | Minimum activity |
| 11 | Buy TX | ≥ 15 | Confirm real buy pressure |
| 12 | Token Age | 4–180 min | Not too new, not expired |
| 13 | Volume | ≥ $5K | Minimum liquidity |
| 14 | Stage | MIGRATED | Only pump.fun graduated tokens |

### Layer 2: Client-Side Pre-filter (`classify_token()`)

| # | Filter | Default | Description |
|---|--------|---------|-------------|
| 15 | B/S Ratio | ≥ 1.3 | buyTxCount1h / sellTxCount1h |
| 16 | Vol/MC Ratio | ≥ 5% | volumeUsd1h / marketCapUsd |
| 17 | Top10 (recheck) | ≤ 55% | Second pass (5% tolerance) |

### Layer 3: Deep Safety Check (`deep_safety_check()`)

| # | Filter | Default | Source |
|---|--------|---------|--------|
| 18 | Dev Rug Count | = 0 (ZERO tolerance) | tokenDevInfo |
| 19 | Dev Total Launches | ≤ 20 | tokenDevInfo |
| 20 | Dev Holding % | ≤ 15% | tokenDevInfo |
| 21 | Bundler ATH % | ≤ 25% | tokenBundleInfo |
| 22 | Bundler Count | ≤ 5 | tokenBundleInfo |

---

## Signal Detection Engine

### Signal A — TX / Volume Acceleration

Approximated from 1-minute candle volume velocity vs previous 5-minute average.

| Param | Normal | Hot Mode |
|-------|--------|----------|
| Ratio threshold | 1.35× | 1.20× |
| Minimum TX floor | 60 projected | 60 projected |

### Signal B — Volume Surge

```
Current 1m candle volume / previous 5m average ≥ threshold
```

| Mode | Threshold |
|------|-----------|
| HOT  | 2.0× |
| QUIET | 1.5× |

### Signal C — Buy Pressure Dominant

```
1h B/S ratio ≥ 1.5
```

### Signal Tiers

| Tier | Condition | Position Size | Slippage |
|------|-----------|---------------|----------|
| **SCALP** | Signal A + Signal C | 0.0375 SOL | 8% |
| **MINIMUM** | Signal A + Signal B + Signal C | 0.075 SOL | 10% |

### Launch Classification

| Type | Condition | SL | Time Stop |
|------|-----------|-----|-----------|
| HOT | Last candle volume > $150M | −20% | 8 min |
| QUIET | Everything else | −25% | 15 min |

---

## Cost Model

```
breakeven_pct = (FIXED_COST_SOL / sol_amount) × 100 + COST_PER_LEG_PCT × 2

SCALP   (0.0375 SOL): 0.001/0.0375×100 + 1.0×2 = 2.7% + 2.0% = 4.7%
MINIMUM (0.075  SOL): 0.001/0.075×100  + 1.0×2 = 1.3% + 2.0% = 3.3%
```

| Param | Value |
|-------|-------|
| `FIXED_COST_SOL` | 0.001 (priority_fee×2 + rent) |
| `COST_PER_LEG_PCT` | 1.0% (gas + slippage + DEX fee) |

---

## 8-Layer Exit System (Priority Order)

| Layer | Exit Type | Trigger | Action |
|-------|-----------|---------|--------|
| 1 | Emergency | pnl ≤ −50% | Sell 100% |
| 2 | Stop Loss | pnl ≤ sl_pct (tier/launch dependent) | Sell 100% |
| 3 | Time Stop | age ≥ s3_min AND TP1 not hit AND pnl < +20% | Sell 100% |
| 4 | Take Profit 1 | pnl ≥ TP1 + breakeven | Sell 40–60% (tier/launch) |
| 5 | Breakeven | TP1 hit AND pnl ≤ 0% | Sell 100% |
| 6 | Trailing Stop | TP1 hit AND price < peak×(1−trailing%) | Sell 100% |
| 7 | Take Profit 2 | TP1 hit AND pnl ≥ TP2 + breakeven | Sell 80–100% |
| 8 | Max Hold | age ≥ 30 min | Sell 100% |

### TP Sell Fractions

| TP Level | SCALP | HOT | QUIET |
|----------|-------|-----|-------|
| TP1 sell % | 60% | 50% | 40% |
| TP2 sell % | 100% | 100% | 80% |

### Stop Loss Values

| Tier/Launch | SL % |
|-------------|------|
| SCALP | −15% |
| HOT | −20% |
| QUIET | −25% |
| Emergency | −50% |

### Time Stops

| Tier | Limit |
|------|-------|
| SCALP | 5 min |
| HOT | 8 min |
| QUIET | 15 min |
| Max Hold (all) | 30 min |

---

## Configurable Parameters

Config file: `~/.plugin-store/memepump_scanner_config.json`
(Note: config is stored relative to the binary's directory in some environments)

### Scan Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `stage` | MIGRATED | Token lifecycle stage to scan |
| `tf_min_mc` | 80000 | Min market cap (USD) |
| `tf_max_mc` | 800000 | Max market cap (USD) |
| `tf_min_holders` | 50 | Min holder count |
| `tf_max_dev_hold` | 10 | Max dev holding % |
| `tf_max_bundler` | 15 | Max bundler % |
| `tf_max_sniper` | 20 | Max sniper % |
| `tf_max_insider` | 15 | Max insider % |
| `tf_max_top10` | 50 | Max top10 holdings % |
| `tf_max_fresh` | 40 | Max fresh wallet % |
| `tf_min_tx` | 30 | Min total TX count |
| `tf_min_buy_tx` | 15 | Min buy TX count |
| `tf_min_age` | 4 | Min token age (minutes) |
| `tf_max_age` | 180 | Max token age (minutes) |
| `tf_min_vol` | 5000 | Min volume (USD) |
| `cf_min_bs_ratio` | 1.3 | Min buy/sell TX ratio |
| `cf_min_vol_mc_pct` | 5.0 | Min vol/MC ratio (%) |
| `cf_max_top10` | 55.0 | Max top10 % (client re-check) |

### Deep Safety

| Parameter | Default | Description |
|-----------|---------|-------------|
| `ds_max_dev_hold` | 15.0 | Max dev holding % |
| `ds_max_bundler_ath` | 25.0 | Max bundler ATH % |
| `ds_max_bundler_count` | 5 | Max bundler count |

### Position Sizing

| Parameter | Default | Description |
|-----------|---------|-------------|
| `scalp_sol` | 0.0375 | SOL per SCALP trade |
| `minimum_sol` | 0.075 | SOL per MINIMUM trade |
| `max_sol` | 0.15 | Max total deployed |
| `max_positions` | 7 | Max concurrent positions |
| `slippage_scalp` | 8 | Slippage % for SCALP |
| `slippage_minimum` | 10 | Slippage % for MINIMUM |

### Exit Rules

| Parameter | Default | Description |
|-----------|---------|-------------|
| `tp1_pct` | 15.0 | TP1 raw % target |
| `tp2_pct` | 25.0 | TP2 raw % target |
| `sl_scalp` | −15.0 | SL for SCALP tier |
| `sl_hot` | −20.0 | SL for HOT launch |
| `sl_quiet` | −25.0 | SL for QUIET launch |
| `trailing_pct` | 5.0 | Trailing stop distance (%) |
| `max_hold_min` | 30 | Hard max hold time (minutes) |

### Session Risk

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_consec_loss` | 2 | Consecutive losses before pause |
| `pause_loss_sol` | 0.05 | Cumulative loss pause threshold |
| `stop_loss_sol` | 0.10 | Cumulative loss stop threshold |
| `tick_interval_secs` | 10 | Scan interval (seconds) |

---

## CLI Command Details

### strategy-memepump-scanner tick

Execute one full scan cycle: fetch token list, check exits, scan new entries, execute trades.

```bash
strategy-memepump-scanner tick [--dry-run]
```

**Return fields:**

| Field | Description |
|-------|-------------|
| `tick_time` | ISO 8601 timestamp |
| `positions` | Number of open positions |
| `session_pnl_sol` | Session PnL in SOL |
| `actions` | Array of actions |
| `dry_run` | Whether this was a dry-run |

**Action types:**
- `buy` — position opened (symbol, tier, launch, sol_amount, price, tx_hash)
- `exit` — position closed (symbol, reason, pnl_sol, pnl_pct, sell_pct, tx_hash)
- `skip` — token rejected (symbol, reason)
- `buy_failed` — swap failed (symbol, error)
- `exit_failed` — sell failed (symbol, error)
- `paused` — session paused (until)
- `session_stop` — session terminated (reason)

### strategy-memepump-scanner status

**Return fields:**

| Field | Description |
|-------|-------------|
| `bot_running` | Whether bot is active |
| `stopped` / `stop_reason` | If stopped by risk control |
| `positions` | Open position details (symbol, tier, launch, entry_price, entry_sol, tp1_hit) |
| `session_pnl_sol` | Session PnL |
| `consecutive_losses` | Current loss streak |
| `paused_until` | Pause expiry (if paused) |

### strategy-memepump-scanner report

**Return fields:**

| Field | Description |
|-------|-------------|
| `total_buys` / `total_sells` | Trade counts |
| `total_invested_sol` / `total_returned_sol` | SOL flows |
| `total_pnl_sol` / `session_pnl_sol` | PnL |
| `win_count` / `loss_count` / `win_rate` | Win/loss stats |
| `signals_total` | Total signals recorded |

---

## State Files

| File | Purpose |
|------|---------|
| `memepump_scanner_state.json` | Full bot state (positions, trades, signals, stats) |
| `memepump_scanner_config.json` | User-configurable parameters |
| `memepump_scanner.pid` | PID file for running bot |

(Stored relative to binary directory)

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| "onchainos wallet not available" | Not logged in | `onchainos wallet login` |
| No buys — all skipped | Filters too strict | Check skip reasons; lower thresholds |
| Circuit breaker trips | 5+ consecutive errors | Check onchainos, wait 1h or `reset --force` |
| "Bot stopped" on tick | Session loss limit hit | `reset --force` |
| Sell fails | Low liquidity | Try `sell-all` retry, or `sell <addr> --amount <raw>` |
| All tokens are QUIET | Normal for low-volume periods | Lower `HOT_VOL_THRESHOLD` or adjust SL |
