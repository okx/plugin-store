# KAIROS Floor — Strategy Reference Guide

## Philosophy

KAIROS FLOOR is built on a single principle: **don't trade for the sake of trading**.
Every trade must pass through 4 gates. Each gate eliminates bad setups.

The market regime gate (Sentinel) is the first filter. If the market is ranging or choppy,
no trade is better than a losing trade. The scanner then finds setups only in the confirmed direction.
The guardian enforces position and capital limits. The executor sizes based on conviction strength.

---

## Indicator Reference

### EMA (Exponential Moving Average)
- **9 EMA**: Short-term momentum. Primary signal on 5m TF.
- **21 EMA**: Medium-term trend. The key line for direction.
- **50 EMA**: Intermediate trend confirmation.
- **Alignment**: 9 > 21 > 50 = bullish trend. Reverse = bearish.

### RSI (Relative Strength Index, period 14)
- **Sweet spot**: 30-70. Trend is healthy and has room to continue.
- **Exit warning**: 69+ (LONG) or 31- (SHORT). Consider tightening.
- **Hard exit**: 78+ (LONG) or 22- (SHORT). Extreme — exit immediately.
- **Entry filter**: RSI must be in 25-75 range (room to run).

### MACD (12/26/9)
- **Histogram > 0**: Bullish momentum. Confirms LONG setups.
- **Histogram < 0**: Bearish momentum. Confirms SHORT setups.
- **Histogram acceleration**: Even stronger signal (histogram increasing).

### ADX (Average Directional Index, period 14)
- **< 20**: No trend. Don't trade directionally.
- **20-35**: Trend developing. Minimal position size.
- **35-50**: Strong trend. Normal sizing.
- **> 50**: Very strong trend. May be overextended.

### ATR (Average True Range, period 14)
- **Primary use**: Stop-loss distance calculation (SL = entry ± ATR × 2.0)
- **Secondary use**: Trailing stop distance (TS = price ± ATR × 1.5)
- **Breakout signal**: If last candle > ATR × 1.5 → momentum confirms direction

### Bollinger Bands (period 20, 2 std dev)
- **Width**: (Upper - Lower) / Middle. Measures volatility.
- **Compression**: Low width = coiling energy. Big move incoming.
- **Signal**: If current width < 40th percentile of recent widths → setup is valid.

---

## Market Regime States

| Regime | Condition | Trade Direction | Size |
|--------|-----------|-----------------|------|
| trending_up | EMA aligned bull + ADX > 20 | LONG only | Normal |
| trending_down | EMA aligned bear + ADX > 20 | SHORT only | Normal |
| ranging | EMA mixed + ADX < 20 | BOTH (careful) | Reduced |
| volatile | ADX > 40 OR RSI > 82/< 18 | Based on trend | Reduced |

---

## Fear & Greed Integration

The Crypto Fear & Greed Index (0-100) modulates regime direction:

| FG Value | Classification | Effect |
|----------|---------------|--------|
| 0-15 | Extreme Fear | Bias LONG (contrarian) |
| 15-30 | Fear | Slight bullish bias |
| 30-70 | Neutral-Greed | No adjustment |
| 70-85 | Greed | Slight bearish bias |
| 85-100 | Extreme Greed | Bias SHORT (contrarian) |

---

## Conviction Scoring

Each setup receives a conviction score (0-100) based on:
- **Confluence factors met** (weighted by count above minimum)
- **Sentinel regime strength** (1-5, added as bonus points)
- **Confidence from scanner** (base signal from factor %)

The score determines the conviction tier which sets leverage and risk%.

High conviction = more leverage + larger position. This maximizes volume on strong setups
while keeping SCOUT-tier trades small on weaker signals.

---

## R-Multiple System

All position management uses R-multiples where:
- **1R = SL distance** (= ATR × 2.0 from entry)

| Milestone | Action |
|-----------|--------|
| +0.3R | Enable RSI tighten monitoring |
| +0.7R | Move SL to breakeven (+0.15%) |
| +1.2R | Activate ATR trailing stop |
| Every $2 | Step-lock profit increment |

---

## Position Sizing Formula

```
riskAmount    = balance × riskPct (from conviction tier)
slPct         = slDistance / entryPrice
positionValue = riskAmount / slPct
size          = positionValue / entryPrice
```

**Example** (STANDARD tier, $1000 balance):
- riskPct = 1.0% → $10 risk
- SL = 0.5% of price → slPct = 0.005
- positionValue = $10 / 0.005 = $2000
- size = $2000 / $67000 = 0.0299 BTC
- Notional = $2000 with 10x leverage, margin = $200

---

## Symbol Selection

Default symbols: **BTC, ETH, SOL**

Selection criteria:
- Sufficient liquidity (avoid exotic pairs)
- Hyperliquid perpetuals market exists
- Low funding rate (< 0.08% against intended direction)

To trade additional symbols:
```
node scripts/config.mjs set-symbols BTC,ETH,SOL,ARB,DOGE
```

---

## Operational Notes

### API Rate Limits
- Hyperliquid: No documented rate limit, but avoid > 10 req/sec
- CoinGecko free tier: 30 req/min. market-data.mjs includes retry logic.
- Fear & Greed: Very permissive, no practical limit.

### Data Requirements (minimum candle counts)
- Sentinel: 30+ candles (15m), 50+ candles (1h)
- Scanner per symbol: 30+ candles (5m, 15m, 1h each)
- ADX: 2× period + 1 = 29+ candles minimum

### Lookback Windows
| TF | Lookback | Candles ~obtained |
|----|----------|-------------------|
| 5m | 12h | ~144 |
| 15m | 48h | ~192 |
| 1h | 7d | ~168 |

These windows ensure enough candles for all indicators.

---

## Logging & Debugging

The engine logs each phase with clear prefixes:
- `[KAIROS]` — Main cycle
- `[SENTINEL]` — Regime detection
- `[SCANNER]` — Setup detection
- `[GUARDIAN]` — Risk validation
- `[EXECUTOR]` — Order execution
- `[DRY-RUN]` — Simulated actions
- `[LIVE]` — Real order actions
- `[RISK]` — Risk manager actions

State is persisted to `.kairos-data/state.json` between cycles.

---

## Challenge Strategy (OKX Plugin Store)

To maximize the 3 leaderboards (volume, tx count, unique addresses):

1. **Volume**: More trades × larger notional. Run 24/7 with auto mode.
2. **TX count**: More trades. Gates are relaxed (2/9 confluence) to generate more signals.
3. **Unique addresses**: Share the strategy. Different wallets using the plugin = more addresses.

The strategy deliberately uses smaller position sizes but higher frequency vs the production KAIROS engine.
A $500 notional trade × 50 trades/day = $25,000 daily volume. Stack from multiple accounts for multiplier effect.
