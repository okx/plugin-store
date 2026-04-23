---
name: whale-tracker
description: "A Hyperliquid perpetual contract strategy Skill based on Smart Money Concepts (SMC). Identifies institutional Order Blocks, Fair Value Gaps, and liquidity sweeps to track whale activity on BTC. Activate when user says: whale tracker, smart money, institution tracking, SMC strategy, or follow the whales."
version: "1.0.0"
author: "kin"
tags:
  - hyperliquid
  - btc
  - perpetual
  - smc
  - smart-money
  - order-block
  - whale
---

# Whale Tracker: Smart Money Footprint Strategy V1.0

> *"In the ocean, small fish don't need to be smarter than whales. They just need to know where the whales are, and swim along."*

---

## Overview

The BTC market is not a playground for retail traders. Every "inexplicable" dump is whales sweeping retail stop-losses. Every "fake breakout" reversal is institutions harvesting late buyers. Every consolidation range is big money quietly building positions.

This strategy has one core belief: **Don't try to predict whales — learn to read the footprints they leave behind.**

Whales cannot hide themselves. Their size means every entry and exit leaves traces on price structure:
- **Order Block (OB)**: The last location where whales made a large buy or sell — price tends to return and retest these zones
- **Fair Value Gap (FVG)**: Whales moved so fast they left a "vacuum" — markets naturally want to fill these
- **Liquidity Sweep**: Whales deliberately push price into retail stop-loss clusters, harvest the liquidity, then reverse sharply

This Skill runs on **Hyperliquid** via the Hyperliquid Plugin. Read the footprints, follow the whales.

---

## Pre-flight Checks

Before using this skill, ensure:

1. The `onchainos` CLI is installed and configured:
   ```bash
   npx skills add okx/onchainos-skills
   export PATH="$HOME/.local/bin:$PATH"
   ```
2. Hyperliquid Plugin is installed:
   ```bash
   npx skills add okx/plugin-store --skill hyperliquid
   ```
3. You have a funded Hyperliquid account connected to Onchain OS
4. Confirm wallet connection:
   ```bash
   onchainos wallet balance --chain hyperliquid
   ```

---

## Risk Disclaimer

⚠️ **IMPORTANT: READ BEFORE USE**

- This is an automated trading strategy operating on perpetual contracts with leverage
- Perpetual contract trading carries **extremely high risk** and may result in total loss of capital
- This Skill is for **educational and research purposes**. It does not constitute financial advice
- Always use **dry-run mode first** to validate strategy behavior before committing real funds
- Never invest more than you can afford to lose completely
- Past performance does not guarantee future results

---

## Dry-Run Mode (Paper Trading — Run This First)

**All new users must test in dry-run mode before live trading.**

To enable dry-run mode, tell the agent:

> "Run whale tracker in dry-run mode"

In dry-run mode, the Skill will:
- Complete all analysis steps (Steps 1–3) and generate full signal output
- **Skip Step 4** (no real orders placed)
- Output: `[DRY-RUN] Would place [BUY/SELL] order at [price], SL: [price], TP1/2/3: [prices]`
- Log all signals for you to review performance over time

Run dry-run for at least **48 hours** before switching to live mode.

---

## Execution Rhythm

Trigger a full scan cycle every **1 hour**, or on-demand when user requests analysis.

---

## Commands

### Step 1 · Footprint Detection: Find Whale Activity Traces

> *"Whales don't speak. But their behavior does."*

#### 1.1 Market Structure (Confirm Whale Direction)

Fetch BTC 4h candles from Hyperliquid to establish macro market structure:

```
Use Hyperliquid Plugin to fetch BTC-USD perpetual 4h OHLCV data (last 100 candles)
```

Identify:
- **Swing High / Swing Low**: Price is the highest/lowest across 5 consecutive candles
- **Trending Structure**: HH/HL (Higher High / Higher Low) = whales buying; LL/LH = whales selling
- **Break of Structure (BOS)**: Price effectively breaks above/below the last swing point = whales changing direction
- **Change of Character (CHoCH)**: Counter-directional break with elevated volume = possible trend reversal

Also fetch **1h candles** for precise entry-level micro structure.

#### 1.2 Order Block Identification (Locate Institutional Order Zones)

Order Blocks are the clearest whale footprints. Identification rules on **4h timeframe**:

- **Bullish OB (Demand Zone)**: Find the last **bearish candle** in a trend, followed immediately by a strong rally (move > 1.5 × ATR14). The high-to-low range of that bearish candle = OB zone
- **Bearish OB (Supply Zone)**: Find the last **bullish candle** in a trend, followed immediately by a strong decline (move > 1.5 × ATR14). That bullish candle's range = OB zone
- OB remains valid until price returns and closes through it (invalidated on close beyond OB)
- Track the **3 most recent valid OBs**; prioritize the one closest to current price

Repeat on **1h timeframe** for precision entry OBs.

#### 1.3 Fair Value Gap Identification (Price Imbalance Zones)

Scan the last 100 **1h candles** for FVGs:

- **Bullish FVG**: Candle N's low > Candle N-2's high (upward price vacuum)
- **Bearish FVG**: Candle N's high < Candle N-2's low (downward price vacuum)
- Minimum FVG size: gap width > 0.3 × ATR14 (filter out noise gaps)
- FVG remains active until price re-enters the zone

#### 1.4 Liquidity Sweep Detection (Whale Harvesting Signal)

Check recent **1h candles** for sweep patterns:

- **Sweep characteristics**:
  - Price briefly breaks beyond a recent Swing High/Low (creates long wick)
  - But the candle **closes back inside** the breakout level (reversal close)
  - Accompanied by elevated volume (> 1.5 × 20-period average volume)
- **Signal meaning**: Whales harvested retail stop-losses at the sweep level, preparing to reverse
- **Signal validity**: Best entry window is within the **next 3 1h candles** after the sweep

**When to use**: Run this full Step 1 analysis at the start of each hourly cycle, or when user asks for a whale activity report.

**Output**: A structured report listing all identified OBs, FVGs, and any sweep signals with their price levels.

---

### Step 2 · Footprint Grading: Assess Whale Confidence

> *"Not every footprint is worth following. A big whale's print is worth more than a small one's."*

Score the current signal set out of **100 points**:

| Signal Dimension | Scoring Criteria | Max Score |
|---|---|---|
| **Market Structure Alignment** | 4h and 1h structures agree: 25pts; Only one timeframe clear: 12pts; Conflicting: 0pts | 25 |
| **Order Block Quality** | OB formed with volume > 2× average: 20pts; 1.5–2×: 12pts; < 1.5×: 5pts | 20 |
| **FVG Present** | Current price inside a valid FVG: 20pts; FVG overlaps with OB (dual confirmation): +5 bonus | 25 |
| **Liquidity Sweep** | Sweep signal within last 3 candles, direction matches: 20pts; Within 4–6 candles: 10pts; None: 0pts | 20 |
| **Funding Rate Health** | Fetch from Hyperliquid Plugin; absolute value < 0.05%: 10pts; < 0.08%: 5pts; ≥ 0.08%: 0pts | 10 |

**Confidence thresholds**:
- **≥ 75 points**: High confidence — standard position size allowed
- **55–74 points**: Medium confidence — halve the position size
- **< 55 points**: Insufficient signal — skip this cycle, wait for next

**Output**: Confidence score with breakdown by dimension.

---

### Step 3 · Whale Intent Reading: AI Synthesis (Core Step)

> *"Technical indicators tell you where price is. Smart money analysis tells you where whales want to go."*

AI must complete full intent reasoning. **No sub-item may be skipped.**

#### 3.1 What Are Whales Doing Right Now?

Based on Step 1 footprint data, determine the most likely whale behavior mode:

- **Accumulation Mode**: Price oscillating within OB zone + mild volume + slight open interest growth → whales quietly building positions
- **Mark-Up / Mark-Down Mode**: Price breaking key structure + volume spike + new FVG forming → whales driving direction
- **Harvest Mode**: Liquidity sweep signal present → whales eliminating retail, preparing to reverse

#### 3.2 What Are Retail Traders Doing? (Contrarian Reference)

Fetch open interest data from Hyperliquid Plugin:

- Open interest surging same direction as price (> 20% / 24h): retail chasing — whales may soon reverse and harvest
- Open interest moving against price: smart money leading — direction signal more reliable

#### 3.3 Final Intent Declaration

AI must output:

```
【Whale Intent Analysis】
Current Footprints: [OB levels / FVG zones / Sweep signal present: Y/N]
Whale Behavior Mode: [Accumulation / Mark-Up / Mark-Down / Harvest]
Retail Sentiment: [Chasing / Neutral / Fearful]
Confidence Score: [X/100]
Direction: [LONG / SHORT]
Core Reasoning: [1-2 sentences explaining why this is whale intent, not a trap]
```

If AI suspects a **whale trap** (e.g., sweep immediately followed by counter-directional large volume, or OB zone tested more than 3 times), **abandon this cycle entry** and note: `"Possible trap detected — skipping this cycle."`

---

### Step 4 · Execute Order (Only When Confidence ≥ 55 and Intent Declaration Complete)

> *"Follow the whale, but don't swim too close — when it rolls over, it will crush you."*

**Skip this step entirely in dry-run mode.**

Place order via Hyperliquid Plugin:

```
Asset: BTC-USD Perpetual
Side: [LONG or SHORT based on Step 3]
Order Type: Market
Size: [calculated from Step 5 risk management]
```

Confirm order placement and log entry price.

**When to use**: Only after Steps 1–3 are complete, confidence ≥ 55, and user has confirmed they want to proceed with live trading.

---

### Step 5 · Stop-Loss and Take-Profit (Structure-Anchored, Not Fixed Percentage)

> *"A stop-loss is not admitting defeat. It's preserving bullets for the next whale track."*

Set stop-loss based on OB structure (not arbitrary percentage):

| Direction | Stop-Loss Level | Logic |
|---|---|---|
| Long | 0.3% below the entry OB's low | OB invalidated if broken = whale intent failed |
| Short | 0.3% above the entry OB's high | OB invalidated if broken = whale intent failed |

**Maximum risk per trade = Account equity × 1.5%** — back-calculate position size from this.
(If confidence is 55–74: maximum risk = Account equity × 0.75%)

Set take-profit in three tiers:

| TP Tier | Target Level | Close Percentage |
|---|---|---|
| TP1 | Midpoint of nearest FVG | 40% of position |
| TP2 | Edge of nearest opposing OB | 40% of position |
| TP3 | Nearest Swing High/Low (liquidity pool) | Remaining 20% |

After TP1 triggers: move stop-loss to **breakeven**.

Place all stops and targets via Hyperliquid Plugin immediately after entry. Stop placement must complete within **90 seconds of entry**.

---

### Step 6 · Position Monitoring

Each hourly cycle checks existing positions for **whale intent reversal signals**:

- **Counter CHoCH detected** (structural change of character against position)? → Reduce position by 50%, tighten stop
- **Counter OB forming** and price is testing it? → Prepare to close full position
- **Funding rate jumps** above 0.1%? → Evaluate early exit

**Output**: Position health report with current P&L, distance to SL/TP levels, and any warning flags.

---

## Risk Controls Summary

```
// [P1] Max single trade risk: 1.5% of equity (0.75% if confidence 55-74)
// [P2] Stop all new entries if daily drawdown exceeds 5%
// [P3] Same OB zone: max 2 entries. 3rd test = no longer trackable
// [P4] Stop placement must complete within 90 seconds of entry
// [P5] Never add to a losing position (most common whale trap victim behavior)
// [P6] Liquidity sweep signal expires after 3 candles — do not chase late entries
// [P7] Dry-run mode must be tested for 48 hours before live trading
```

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| Hyperliquid data fetch fails | Network issue or API limit | Wait 30 seconds and retry; if persistent, skip this cycle |
| Open interest data unavailable | Hyperliquid Plugin connectivity | Proceed without OI data; note it in output and reduce score by 10pts |
| Order rejected: insufficient margin | Account balance too low for calculated size | Reduce position size by 50% and retry once |
| Stop placement timeout (> 90s) | Network congestion | Close position immediately via market order; log as failed trade |
| Confidence score exactly 55 | Borderline signal | Default to half position size; treat as medium confidence |
| OB zone tested 3+ times | Zone reliability degraded | Do not enter; flag zone as exhausted in output |

---

## Skill Routing

- For Polymarket event-driven signals alongside this strategy → use `polymarket` skill
- For wallet balance check before trading → use `onchainos wallet balance`
- For cross-exchange price comparison → use `onchainos market price`
- For security scan on contract address → use `onchainos security token-scan`

---

*Strategy Version: V1.0 | Platform: Hyperliquid via OKX Plugin Store | Track: Quality Skill*
*Inspired by: Smart Money Concepts (SMC) institutional trading theory*
