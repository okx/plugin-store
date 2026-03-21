# Signal Tracker — Strategy & CLI Reference

Complete reference for the 17-point safety filter, position management, cost model, 7-layer exit system, all CLI commands, and configurable parameters.

---

## 17-Point Safety Filter

### Layer 1: Server-Side Pre-filter (4 checks, API parameters)

| # | Filter | Default | Source |
|---|--------|---------|--------|
| 1 | Market Cap | ≥ $200K | signal/list `minMarketCapUsd` |
| 2 | Liquidity | ≥ $80K | signal/list `minLiquidityUsd` |
| 3 | Co-buying wallets | ≥ 3 | signal pre-filter `triggerWalletCount` |
| 4 | Smart money holding | soldRatioPercent < 80% | signal pre-filter |

### Layer 2: Client-Side Deep Verification (13 checks, 6-8 API calls)

| # | Filter | Default | Source |
|---|--------|---------|--------|
| 5 | Market Cap (recheck) | ≥ $200K | price_info |
| 6 | Liquidity (recheck) | ≥ $80K | price_info |
| 7 | Holders | ≥ 300 | price_info |
| 8 | Liq/MC Ratio | ≥ 5% | price_info |
| 9 | Top10 Holder % | ≤ 50% | price_info |
| 10 | Holder Density | ≥ 300 per $1M MC | price_info |
| 11 | LP Burn | ≥ 80% | price_info |
| 12 | 1min K-line Pump | ≤ 15% | candles(1m) |
| 13 | Dev Rug Count | = 0 (ZERO tolerance) | memepump/tokenDevInfo |
| 14 | Dev Launches | ≤ 20 | memepump/tokenDevInfo |
| 15 | Dev Holding % | ≤ 15% | memepump/tokenDevInfo |
| 16 | Bundler ATH % | ≤ 25% | memepump/tokenBundleInfo |
| 17 | Honeypot Check | isHoneyPot=false, taxRate ≤ 5% | swap quote |

---

## Position Sizing (Tiered by Signal Strength)

| Tier | Condition | Position Size |
|------|-----------|---------------|
| **high** | ≥ 8 co-buying wallets | 0.020 SOL |
| **mid** | ≥ 5 co-buying wallets | 0.015 SOL |
| **low** | ≥ 3 co-buying wallets | 0.010 SOL |

| Param | Value |
|-------|-------|
| Max Positions | 6 |
| Slippage | 1% |
| Gas Reserve | 0.05 SOL |

---

## Cost Model (Breakeven Calculation)

```
breakeven_pct = (FIXED_COST_SOL / position_sol) × 100 + COST_PER_LEG_PCT × 2

high (0.020):  0.001/0.020×100 + 1.0×2 = 5.0% + 2.0% = 7.0%
mid  (0.015):  0.001/0.015×100 + 1.0×2 = 6.7% + 2.0% = 8.7%
low  (0.010):  0.001/0.010×100 + 1.0×2 = 10.0% + 2.0% = 12.0%
```

| Param | Value | Description |
|-------|-------|-------------|
| `FIXED_COST_SOL` | 0.001 | priority_fee×2 + rent (round trip) |
| `COST_PER_LEG_PCT` | 1.0% | gas + slippage + DEX fee per leg |

---

## 7-Layer Exit System (Priority Order)

| Layer | Exit Type | Trigger | Sell % |
|-------|-----------|---------|--------|
| 0 | RUG_LIQ | liquidity < $5K | 100% |
| 1 | Dust | position value < $0.10 | 100% |
| 2 | TIME_DECAY_SL | 15min+: pnl ≤ −10%; 30min+: pnl ≤ −8%; 60min+: pnl ≤ −5% | 100% |
| 3 | HARD_SL | pnl ≤ −10% (SL_MULTIPLIER = 0.90) | 100% |
| 4 | TP1/TP2/TP3 | cost-aware net targets (see table below) | Partial |
| 5 | TRAILING_STOP | TP1 hit + peak pnl drops 10% | 100% |
| 6 | TIME_STOP | hold ≥ 4 hours | 100% |

### Take-Profit (Cost-Aware Net Targets)

| Tier | Net Target | Sell % | Raw trigger (low) | Raw trigger (high) |
|------|-----------|--------|-------------------|--------------------|
| TP1 | +5% net | 30% | 5% + 12% = **17%** | 5% + 7% = **12%** |
| TP2 | +15% net | 40% | 15% + 12% = **27%** | 15% + 7% = **22%** |
| TP3 | +30% net | 100% | 30% + 12% = **42%** | 30% + 7% = **37%** |

### Trailing Stop

| Param | Value |
|-------|-------|
| Activate | After TP1 hit AND pnl ≥ +12% |
| Distance | 10% drawdown from peak pnl |

### Time-Decay Stop Loss

Active only before any TP is triggered (tp_tier == 0):

| Hold Time | SL Level |
|-----------|----------|
| ≥ 15 min | −10% |
| ≥ 30 min | −8% (tightens) |
| ≥ 60 min | −5% (further tightens) |

---

## Configurable Parameters

Config file: `~/.plugin-store/signal_tracker_config.json`

### Signal Filter

| Parameter | Default | Description |
|-----------|---------|-------------|
| `signal_labels` | "1,2,3" | 1=SmartMoney, 2=KOL, 3=Whale |
| `min_wallet_count` | 3 | Minimum co-buying wallets |
| `max_sell_ratio` | 0.80 | Skip if smart money sold > 80% |

### Safety Thresholds

| Parameter | Default | Description |
|-----------|---------|-------------|
| `min_mcap` | 200000 | Minimum market cap (USD) |
| `min_liquidity` | 80000 | Minimum liquidity (USD) |
| `min_holders` | 300 | Minimum holder count |
| `min_liq_mc_ratio` | 0.05 | Minimum liq/MC ratio |
| `max_top10_holder_pct` | 50.0 | Maximum top10 holder % |
| `min_lp_burn` | 80.0 | Minimum LP burn % |
| `min_holder_density` | 300.0 | Minimum holders per $1M MC |
| `max_k1_pump_pct` | 15.0 | Max 1m pump % at entry |

### Dev/Bundler

| Parameter | Default | Description |
|-----------|---------|-------------|
| `dev_max_launched` | 20 | Max dev launched tokens |
| `dev_max_hold_pct` | 15.0 | Max dev holding % |
| `bundle_max_ath_pct` | 25.0 | Max bundler ATH % |
| `bundle_max_count` | 5 | Max bundler count |

### Position Sizing

| Parameter | Default | Description |
|-----------|---------|-------------|
| `position_high_sol` | 0.020 | SOL per trade (high tier) |
| `position_mid_sol` | 0.015 | SOL per trade (mid tier) |
| `position_low_sol` | 0.010 | SOL per trade (low tier) |
| `wallet_high_threshold` | 8 | Wallets for high tier |
| `wallet_mid_threshold` | 5 | Wallets for mid tier |
| `max_positions` | 6 | Max concurrent positions |
| `slippage_pct` | "1" | Swap slippage (%) |
| `gas_reserve_sol` | 0.05 | SOL reserved for gas |

### Cost Model

| Parameter | Default | Description |
|-----------|---------|-------------|
| `fixed_cost_sol` | 0.001 | Fixed cost per round trip |
| `cost_per_leg_pct` | 1.0 | Cost per swap leg (%) |

### Take Profit

| Parameter | Default | Description |
|-----------|---------|-------------|
| `tp1_pct` | 5.0 | TP1 net target % |
| `tp1_sell` | 0.30 | TP1 sell fraction |
| `tp2_pct` | 15.0 | TP2 net target % |
| `tp2_sell` | 0.40 | TP2 sell fraction |
| `tp3_pct` | 30.0 | TP3 net target % |
| `tp3_sell` | 1.00 | TP3 sell fraction |

### Trailing Stop

| Parameter | Default | Description |
|-----------|---------|-------------|
| `trail_activate_pct` | 12.0 | Activation threshold (%) |
| `trail_distance_pct` | 10.0 | Stop distance from peak (%) |

### Stop Loss & Time

| Parameter | Default | Description |
|-----------|---------|-------------|
| `sl_multiplier` | 0.90 | Hard SL (−10%) |
| `liq_emergency` | 5000.0 | Emergency exit liquidity threshold (USD) |
| `time_stop_hours` | 4.0 | Hard time stop (hours) |

### Session Risk

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_consec_loss` | 3 | Consecutive losses before pause |
| `pause_consec_sec` | 600 | Pause duration for consecutive losses (10 min) |
| `session_loss_limit_sol` | 0.05 | Cumulative loss pause threshold |
| `session_loss_pause_sec` | 1800 | Pause duration for loss limit (30 min) |
| `session_stop_sol` | 0.10 | Cumulative loss stop threshold |
| `tick_interval_secs` | 20 | Signal poll interval (seconds) |

### Circuit Breaker

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_consecutive_errors` | 5 | Errors before circuit breaker trips |
| `cooldown_after_errors` | 3600 | Cooldown after circuit breaker (1h) |

---

## CLI Command Details

### strategy-signal-tracker tick

Execute one full tick: fetch signals, check exits on all positions, open new positions.

```bash
strategy-signal-tracker tick [--dry-run]
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
- `buy` — position opened (symbol, label, tier, sol_amount, price, wallet_count, amount_raw)
- `exit` — position closed/partial (symbol, reason, pnl_sol, pnl_pct, net_pnl_pct, sell_pct, tx_hash)
- `skip` — token rejected (symbol, reason)
- `buy_failed` — swap failed (symbol, error)
- `exit_failed` — sell failed (symbol, error)
- `paused` — session paused (until timestamp)
- `session_stop` — session terminated (reason)
- `max_positions_reached` — no new buys

### strategy-signal-tracker status

**Return fields:**

| Field | Description |
|-------|-------------|
| `bot_running` | Whether bot is active |
| `stopped` / `stop_reason` | If stopped by risk control |
| `positions` | Open position details (symbol, label, tier, buy_price, buy_amount_sol, tp_tier, trailing_active, peak_pnl_pct) |
| `known_tokens` | Count of seen tokens |
| `session_pnl_sol` | Session PnL |
| `consecutive_losses` | Current loss streak |
| `cumulative_loss_sol` | Total session losses |
| `paused_until` | Pause expiry timestamp (if paused) |

### strategy-signal-tracker report

**Return fields:**

| Field | Description |
|-------|-------------|
| `total_buys` / `total_sells` | Trade counts |
| `total_invested_sol` / `total_returned_sol` | SOL flows |
| `total_pnl_sol` / `session_pnl_sol` | PnL |
| `win_count` / `loss_count` / `win_rate` | Win/loss stats |
| `positions` | Current open positions count |

---

## Signal Labels

| Label | Type | Description |
|-------|------|-------------|
| 1 | SmartMoney | Wallets with consistent profitable trading history |
| 2 | KOL | Key opinion leaders / influencer wallets |
| 3 | Whale | Large capital wallets |

---

## State Files

| File | Purpose |
|------|---------|
| `~/.plugin-store/signal_tracker_state.json` | Full bot state (positions, trades, stats, known tokens) |
| `~/.plugin-store/signal_tracker_config.json` | User-configurable parameters |
| `~/.plugin-store/signal_tracker.pid` | PID file for running bot |
| `~/.plugin-store/signal_tracker.log` | Execution log |

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| "onchainos wallet not available" | Not logged in | `onchainos wallet login` |
| No buys — all signals skipped | Filters too strict or signal dry spell | Check skip reasons, lower `min_wallet_count` |
| k1 pump rejection | Token pumping at entry | Normal — protects from buying tops |
| Circuit breaker trips | 5+ consecutive errors | Check onchainos connectivity, wait 1h or `reset --force` |
| "Bot stopped" on tick | Session loss limit hit | `reset --force` |
| High breakeven | Small position size | Expected — low tier has 12% breakeven |
| TP1 triggers but position still open | Partial sell (30%) | Remaining 70% continues to TP2/TP3/trailing |
| Sell fails with low liquidity | RUG or thin pool | `sell-all` will retry, or `sell <addr> --amount <raw>` |
