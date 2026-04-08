---
name: kairos-floor-score
version: 1.0.0
description: "Institutional-grade ML trade scoring for AI agents. Regime classification, microstructure analysis, confluence scoring, GO/REDUCE/REJECT recommendation. Trigger on: trade scoring, market analysis, regime detection, risk assessment, entry quality, VPIN, order flow."
author: Valisthea
---

# Kairos Floor Score

## When to Use

Invoke this skill whenever you need to determine the quality of a trade entry. Common triggers:

- "Should I go long/short on X?"
- "Score this trade setup"
- "What's the market regime right now?"
- "Is there enough liquidity to enter?"
- "Check confluence for this entry"
- "What's the VPIN / order flow saying?"
- "Evaluate this trade for risk"

## How to Call

### Full Trade Scoring

```bash
kairos-floor-score analyze \
  --symbol BTC-USDT \
  --side long \
  --klines '<json array of {o,h,l,c,v} candles>' \
  --funding-rate 0.0001 \
  --output json
```

The output JSON contains:
- `recommendation`: `GO` (score >= 0.55), `REDUCE` (>= 0.45), or `REJECT` (< 0.45)
- `score`: ML probability [0, 1]
- `confidence`: HIGH / MEDIUM / LOW
- `regime`: market regime classification with tradability assessment
- `confluence`: 5-group weighted confluence breakdown
- `microstructure`: Kyle Lambda, VPIN, order imbalance, spread

### Regime Detection Only

```bash
kairos-floor-score regime --klines '<json>'
```

### Microstructure Only

```bash
kairos-floor-score microstructure --klines '<json>'
```

### Health Check

```bash
kairos-floor-score health
```

## Interpreting Results

### Recommendation

| Value   | Score Range | Action |
|---------|-------------|--------|
| GO      | >= 0.55     | Execute the trade with normal sizing |
| REDUCE  | 0.45 - 0.55 | Trade with reduced size or wait for better setup |
| REJECT  | < 0.45      | Do not enter this trade |

### Regime

The regime classifier identifies 5 states: `trending_up`, `trending_down`, `ranging`, `volatile`, `dead`. Only trade when `tradable: true`.

### Confluence

5 factor groups are scored independently:
- **Trend** (25%): EMA cross, SMA position, multi-timeframe alignment
- **Momentum** (20%): RSI, ADX, MACD histogram
- **Volatility** (20%): ATR percentage, Bollinger bandwidth
- **Flow** (20%): Volume ratio, order imbalance
- **Sentiment** (15%): Funding rate, premium/discount

### Microstructure

- **VPIN > 0.7**: High probability of informed trading — caution
- **Kyle Lambda high**: Thin liquidity, expect slippage
- **Order Imbalance > 0.3**: Strong directional pressure

## Integration with OKX Onchain OS

If `onchainos` CLI is installed, data can be fetched directly:

```bash
kairos-floor-score analyze \
  --symbol BTC-USDT \
  --side long \
  --source onchainos \
  --chain solana
```

## Agent Workflow

1. Receive trade signal from upstream strategy
2. Call `kairos-floor-score analyze` with current market data
3. If recommendation is `GO`, proceed to position sizing
4. If `REDUCE`, cut position size by 50%
5. If `REJECT`, skip this trade entirely
6. Log the full score result for post-trade analysis
