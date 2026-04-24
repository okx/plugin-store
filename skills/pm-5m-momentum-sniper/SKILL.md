---
name: pm-5m-momentum-sniper
description: "Momentum-driven 5-minute crypto prediction sniper for Polymarket with auto direction signal and continuous session loop"
version: "1.0.0"
author: "awan"
tags:
  - polymarket
  - prediction-market
  - crypto
  - momentum
  - five-minute
---

# PM 5M Momentum Sniper

## Overview

Momentum-driven trading skill for Polymarket 5-minute crypto Up/Down markets. Analyzes recent price action across multiple timeframes to generate a directional signal (Up or Down), then executes trades only when the signal aligns with favorable odds. Supports continuous multi-round sessions with built-in risk controls.

Unlike manual co-pilots that require the user to pick a direction each round, this skill provides a momentum-based signal so the user can run semi-automated sessions with minimal input.

### When This Strategy Works

This strategy relies on short-term price momentum. It performs best during periods of **active market movement**:

- **High activity windows**: US market open/close (9:30AM / 4:00PM ET), Asian market open (9:00PM ET), major economic data releases
- **Trending markets**: When BTC or altcoins are making sustained directional moves (RSI > 60 or < 40)
- **News-driven volatility**: CPI releases, FOMC decisions, large liquidation events, exchange listings

**During sideways/low-volatility periods**, the signal engine will correctly output "skip" for most rounds. This is by design — the strategy avoids trading when there is no statistical edge. Expect 50-80% skip rate during quiet markets. The strategy preserves capital during choppy conditions and deploys it aggressively when momentum is clear.

### Deployment Limitations

This skill runs as an AI agent instruction set (SKILL.md), executed by Claude Code, Cursor, or other compatible AI agents. It requires:

- An active AI agent session to interpret and execute commands
- `onchainos` CLI with an authenticated wallet session
- `polymarket-plugin` binary available in PATH

**This skill cannot run unattended on a server as a standalone bot.** The AI agent must be active to read the SKILL.md, make decisions, and call commands. However, the signal engine binary (`pm-5m-signal`) can be called independently from scripts or cron jobs for signal-only monitoring:

```bash
# Example: cron job to log signals every 5 minutes
*/5 * * * * pm-5m-signal --coin BTC --format json >> /var/log/pm-signals.json

# Example: alert script that notifies when a strong signal appears
pm-5m-signal --coin BTC --budget 50 --token-price 0.50 --format json | \
  python3 -c "import json,sys; d=json.load(sys.stdin); print(d) if d['confidence']>0.5 else None"
```

For semi-automated operation, you can keep a Claude Code session open and tell the agent "Run momentum sniper on BTC" — it will loop through rounds with your confirmation each trade.

## How It Works — Fund Flow

Polymarket runs on Polygon (chain 137). The collateral token is **USDC.e** (Polygon USDC, contract `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174`).

### Where the money comes from

```
Your Polygon wallet (USDC.e)
    │
    ▼  polymarket-plugin deposit --amount 50 --confirm
Polymarket Proxy Wallet (USDC.e)
    │
    ▼  polymarket-plugin buy --amount 5 ...
Outcome Tokens (Up or Down shares)
    │
    ▼  Market resolves after 5 minutes
    ├─ WIN  → shares worth $1 each → polymarket redeem → USDC.e back to proxy
    └─ LOSE → shares worth $0 → nothing to redeem
```

**Funding steps:**
1. User needs USDC.e on Polygon. Sources: bridge from Ethereum/Arbitrum, withdraw from CEX to Polygon, or swap on Polygon DEX
2. `polymarket-plugin deposit --amount <usdc> --confirm` moves USDC.e from EOA wallet into the Polymarket proxy wallet
3. Each `buy` command spends USDC.e from the proxy wallet to purchase outcome tokens
4. Winning tokens are redeemed back to USDC.e via `polymarket-plugin redeem`
5. `polymarket-plugin withdraw --amount <usdc> --confirm` moves USDC.e back to EOA wallet when done

### What you're actually buying

When you `buy --outcome up --amount 5`:
- You spend 5 USDC.e
- You receive Up tokens. Quantity = 5 / token_price (e.g., price 0.50 → you get 10 shares)
- If BTC goes up in 5 minutes: each share is worth $1 → you redeem 10 × $1 = $10 → profit $5
- If BTC goes down: each share is worth $0 → you lose the $5 you spent

### Trading modes

| Mode | How it works | Gas cost |
|------|-------------|----------|
| **EOA** (default) | Trade directly from wallet | Each buy needs a USDC.e approval tx (costs POL gas) |
| **POLY_PROXY** (recommended) | One-time proxy deployment | ~$0.01 POL once, then all trades are gasless (relayer-paid) |

Always use POLY_PROXY mode for this strategy — it eliminates per-trade gas friction.

---

## Pre-flight Checks

Before starting, run these checks in order. Stop if any check fails.

### 0. Install Required Basic Skill

This strategy depends on `polymarket-plugin` (the Polymarket Basic Skill). It must be installed first:

```bash
npx skills add okx/plugin-store --skill polymarket-plugin
```

Verify it is available by running:

```bash
polymarket-plugin --version
```

If the command is not found, the plugin is not installed. Install it before proceeding.

### 1. Jurisdiction Check

```bash
polymarket-plugin check-access
```

If blocked, inform the user and stop. Do not proceed.

### 2. Wallet and Balance Check

```bash
polymarket-plugin quickstart
```

Verify:
- Wallet is connected
- USDC.e balance is sufficient (minimum 30 USDC recommended for a session)
- If balance < 30 USDC, warn the user and suggest depositing more before starting

Check exact balances:

```bash
polymarket-plugin balance
```

This shows POL (for gas) and USDC.e balances for both EOA and proxy wallets.

### 3. Proxy Mode Setup

```bash
polymarket-plugin switch-mode --mode POLY_PROXY
```

POLY_PROXY mode eliminates per-trade gas costs. If the user has not set up a proxy, run:

```bash
polymarket-plugin setup-proxy --confirm
```

This costs ~$0.01 POL one-time.

### 4. Deposit to Proxy (if needed)

If proxy wallet USDC.e balance is insufficient:

```bash
polymarket-plugin deposit --amount <usdc> --confirm
```

This moves USDC.e from the user's EOA wallet into the Polymarket proxy wallet for trading.

---

## Core Strategy Logic

### Signal Engine (Binary)

This skill includes a compiled signal engine `pm-5m-signal` that fetches real-time 1-minute kline data from Binance public API and computes technical indicators to generate a directional signal.

**Run the signal engine:**

```bash
pm-5m-signal --coin BTC --format json
```

**Output example:**

```json
{
  "coin": "BTC",
  "direction": "up",
  "confidence": 0.72,
  "score": 3.15,
  "indicators": {
    "rsi": 62.3,
    "ema_cross_bps": 12.5,
    "momentum_pct": 0.15,
    "volume_ratio": 1.4,
    "price": 93500.00
  },
  "reasoning": "RSI 62.3 bullish; EMA5>EMA15 by 12.5bps bullish; Momentum +0.150% bullish; Volume 1.4x confirms trend"
}
```

**Signal fields:**
- `direction`: `"up"` / `"down"` / `"skip"` — the recommended bet direction
- `confidence`: 0.0 to 1.0 — how strong the signal is
- `score`: raw composite score (positive = up, negative = down, near zero = skip)
- `indicators`: the 4 technical indicators used

### How the Signal Works — 4 Indicators

The engine fetches the last 30 one-minute candles from Binance and computes:

**1. RSI(14) — Relative Strength Index**
- Measures whether the price is overbought or oversold over the last 14 candles
- RSI > 65 → bullish momentum (price has been rising, likely continues short-term)
- RSI < 35 → bearish momentum
- 35-65 → neutral, no signal from this indicator
- Score contribution: -2 to +2

**2. EMA(5) vs EMA(15) Crossover**
- Compares fast moving average (5-candle) against slow moving average (15-candle)
- Fast EMA above slow EMA → short-term uptrend
- Fast EMA below slow EMA → short-term downtrend
- Measured in basis points relative to current price
- Score contribution: -2 to +2

**3. 5-Candle Momentum**
- Simple price change rate over the last 5 minutes
- Positive momentum → price trending up
- Negative momentum → price trending down
- Score contribution: -2 to +2

**4. Volume Confirmation**
- Compares recent 5-candle volume to the prior 5-candle volume
- Volume ratio > 1.3 with existing signal → amplifies signal by 20% (trend has conviction)
- Volume ratio < 0.7 with existing signal → dampens signal by 20% (trend is weak)

**Composite scoring:**
- All 4 indicators are scored and summed (range: roughly -7 to +7)
- Score > +1.0 → direction = "up"
- Score < -1.0 → direction = "down"
- Between -1.0 and +1.0 → direction = "skip" (no clear edge)
- Confidence = |score| / 6.0, capped at 1.0

### Decision Logic — Signal + Odds Filter

The signal engine provides the direction. The SKILL.md then combines it with Polymarket odds:

```
1. Run: pm-5m-signal --coin <COIN> --format json
   → Get direction + confidence

2. IF direction == "skip":
   → Skip this round. No trade.

3. IF direction == "up" or "down":
   → Run: polymarket-plugin get-market --market-id <active_market_id>
   → Read the token price for the signal direction

4. Apply odds filter:
   - confidence >= 0.5 AND token price <= 0.58 → TRADE
   - confidence >= 0.7 AND token price <= 0.62 → TRADE (strong signal gets relaxed threshold)
   - Otherwise → SKIP (odds too expensive for the signal strength)
```

This two-layer filter means a trade only happens when BOTH the technical signal AND the market odds are favorable.

### Position Sizing — Half-Kelly

The signal engine includes a Kelly Criterion-based position sizer. Pass `--budget` and `--token-price` to get a suggested stake:

```bash
pm-5m-signal --coin BTC --budget 50 --token-price 0.48 --format json
```

The `sizing` field in the output contains:

```json
{
  "stake": 7.50,
  "kelly_raw": 0.300,
  "kelly_half": 0.150,
  "fraction": 0.150,
  "reason": "kelly=0.300, half-kelly=0.150, fraction=15.0% of $50"
}
```

**How it works:**

- Kelly formula: `f* = (b × p - q) / b` where `b` = net odds from token price, `p` = confidence, `q` = 1 - p
- Half-Kelly: divide by 2 for safety (reduces variance at the cost of slightly lower expected growth)
- Cap at 25% of budget per round (never risk more than a quarter)
- Minimum stake: $2 (below this, fees eat the edge)
- If Kelly value is zero or negative: stake = $0, skip the trade (no edge exists)

**Examples at budget=$50:**

| Confidence | Token Price | Suggested Stake | Why |
|-----------|-------------|----------------|-----|
| 35% | 0.45 | $0 (skip) | Negative edge, Kelly < 0 |
| 50% | 0.45 | $2.27 | Tiny edge, 4.5% of budget |
| 65% | 0.50 | $7.50 | Good edge, 15% of budget |
| 80% | 0.50 | $12.50 | Strong edge, capped at 25% |

---

## Commands

### 1. Start a Session — `sniper-session`

The user says something like: "Run momentum sniper on BTC" or "Start 5m sniper for ETH"

**Execution flow:**

1. Run all pre-flight checks (jurisdiction, wallet, proxy)
2. Ask the user to confirm session parameters:
   - **Coin**: BTC, ETH, SOL, DOGE, BNB, XRP
   - **Session budget**: Default 50 USDC (range: 20–200 USDC)
   - **Max consecutive losses before pause**: Default 3
   - **Sizing mode**: "kelly" (default, auto-calculated) or "fixed" (user sets a fixed stake)
3. After user confirms, begin the loop

**Session loop (repeat until budget exhausted or user stops):**

```
Round N:
├─ 1. pm-5m-signal --coin <COIN> --budget <remaining_budget> --token-price <from_step_2> --format json
│     → Get direction, confidence, score, indicators, sizing
│
├─ 2. IF direction == "skip":
│     → Print "⏭️ Round N skipped — signal too weak (score: X, RSI: Y)"
│     → Wait for next market
│
│  IF direction == "up" or "down":
│     → polymarket-plugin list-5m --coin <COIN> --count 3
│     → Identify the active (unresolved) market
│     → polymarket-plugin get-market --market-id <active_market_id>
│     → Read token price for signal direction
│     → Apply odds filter (confidence + price threshold)
│
├─ 3. IF edge detected (signal + odds both pass):
│     → Present to user:
│       "📊 Round N | Signal: UP (conf 72%) | RSI 62 | EMA +12bps | Price: 0.52 | Stake: $7.50 (half-kelly)"
│       "Confirm? (yes/no/stop)"
│     → On "yes":
│       polymarket-plugin buy --market-id <id> --outcome up --amount <kelly_stake> --strategy-id pm-5m-momentum-sniper --confirm
│     → On "no": skip this round
│     → On "stop": end session
│
│  IF odds too expensive:
│     → Print "⏭️ Round N skipped — signal UP but price 0.63 too expensive for confidence 0.45"
│
├─ 4. Monitor position until resolution (~5 minutes)
│     → Every ~60 seconds: polymarket get-market --market-id <id>
│     → Check held token price:
│       - If price < 0.25 and > 2 min remaining → suggest early exit (stop-loss)
│       - If price > 0.80 → suggest profit lock (take-profit)
│     → When position shows redeemable:
│       polymarket redeem --market-id <id> --confirm
│
├─ 5. Report round result:
│     "✅ Round N: WON +$0.48" or "❌ Round N: LOST -$5.00"
│     "Session P&L: +$3.20 | Budget remaining: $42.00 | Record: 4W-2L"
│
└─ 6. Risk checks before next round:
      → If consecutive losses ≥ max_consecutive_losses: pause 1 round
      → If remaining budget < stake: end session
      → Otherwise: proceed to Round N+1
```

### 2. Quick Single Round — `sniper-once`

The user says: "Snipe one round on BTC" or "One shot ETH 5m"

Same logic as session loop but executes exactly one round, then stops. Good for users who want to test before committing to a session.

### 3. Check Signal Only — `sniper-signal`

The user says: "What's the momentum signal for BTC?" or "Check BTC 5m signal"

Runs the signal engine and reports:

```bash
pm-5m-signal --coin BTC --format text
```

Output:
```
==================================================
  BTC 5M Signal: ▲ UP (72% confidence)
==================================================
  Price:      $93,500.00
  RSI(14):    62.3
  EMA cross:  +12.5 bps
  Momentum:   +0.1500%
  Volume:     1.4x
  Score:      +3.15
==================================================
  RSI 62.3 bullish; EMA5>EMA15 by 12.5bps bullish; Momentum +0.150% bullish; Volume 1.4x confirms trend

  >> Recommend: BET UP
```

No trade is placed. Information only.

### 4. Session Status — `sniper-status`

The user says: "How's my sniper session going?"

```bash
polymarket-plugin get-positions
polymarket-plugin balance
```

Report: current open positions, session P&L, win/loss record, remaining budget.

---

## Risk Management Rules

These rules are MANDATORY. Never bypass them.

1. **Per-round cap**: Never exceed the user's configured stake per round
2. **Session budget**: Track cumulative spend. Stop when budget is exhausted
3. **Consecutive loss pause**: After N consecutive losses (default 3), skip one round automatically. Print "⚠️ Cooling down — 3 consecutive losses. Skipping one round."
4. **Odds ceiling**: Never buy an outcome priced above 0.62, regardless of signal strength
5. **No chasing**: After a loss, do NOT increase stake. Keep it constant
6. **User confirmation**: Every trade requires explicit user confirmation before execution. Never place a trade without the user saying "yes" or equivalent
7. **Strategy attribution**: Always include `--strategy-id pm-5m-momentum-sniper` on every buy and sell command
8. **Daily loss limit**: If cumulative session losses reach 20% of the initial session budget, end the session automatically. Print "🛑 Daily loss limit reached. Session ended to protect capital."

### Mid-Round Early Exit (Stop-Loss)

5-minute markets are short, but the user can still exit early by selling shares before resolution. This is useful when the market moves against the position and the user wants to cut losses.

**How it works:**

After buying, the outcome token price fluctuates in real-time based on market activity. If the position is losing, the token price drops — but it may still be above $0. Selling early recovers partial value instead of waiting for a potential total loss.

**Early exit flow:**

```
After buy, if user says "cut my losses" or "exit early":

1. polymarket-plugin get-market --market-id <id>
   → Check current price of the held outcome token
   → Check best_bid and liquidity

2. IF best_bid > 0 AND liquidity is sufficient:
   → Calculate loss: (buy_price - current_bid) × shares
   → Present to user:
     "📉 Early exit: bought Up at 0.52, current bid 0.35
      Shares: 9.6 | Recovery: ~$3.36 of $5.00 | Loss: ~$1.64
      Sell now? (yes/no)"

3. IF user confirms:
   polymarket-plugin sell --market-id <id> --outcome up --shares <all_shares> --strategy-id pm-5m-momentum-sniper --confirm

4. Report:
   "🔻 Early exit executed. Recovered $3.36. Loss limited to $1.64 (vs potential $5.00)"
```

**When to suggest early exit:**

The agent should proactively suggest an early exit if ALL of these conditions are met:
- The held outcome token price has dropped below 0.25 (was bought above 0.45)
- There is sufficient liquidity (best_bid exists and spread < 0.15)
- More than 2 minutes remain before market resolution

Present it as: "⚠️ Your Up position dropped to 0.22. You can sell now to recover ~$2.10 instead of risking full loss. Exit early? (yes/no)"

The user always has final say. Never force-sell.

### Profit Lock (Take-Profit)

If the held outcome token price rises significantly before resolution, the user can lock in profit early:

```
After buy, if held token price rises above 0.80:

→ Present to user:
  "📈 Your Up position is now at 0.85 (bought at 0.52)
   Shares: 9.6 | Current value: ~$8.16 | Profit if sold now: ~$3.16
   Lock profit? (yes/no/wait)"

→ "yes": sell immediately
→ "no" or "wait": hold until resolution (potential $1.00 per share if correct)
```

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `check-access` returns blocked | User in restricted jurisdiction | Inform user, stop session |
| Insufficient USDC.e balance | Not enough funds | Suggest `polymarket-plugin deposit` from Polygon wallet |
| `list-5m` returns no active markets | No 5M market currently open for this coin | Wait 1-2 minutes and retry; if persistent, try a different coin |
| `buy` returns minimum size error | Stake below market minimum | Increase stake to meet minimum shown in error |
| `buy` returns "market not accepting orders" | Market too close to settlement | Skip this round, wait for next market |
| `get-market` shows no bids/asks | Market has no liquidity | Skip this round |
| `sell` fails with "insufficient shares" | Share count mismatch | Re-check positions with `get-positions`, use exact share count |
| `sell` fails with no best_bid | No buyer available for early exit | Cannot exit early; hold until resolution |
| `redeem` fails | Market not yet resolved | Wait and retry after 1 minute |
| Price drifted significantly between signal and execution | Market moved during confirmation | Re-check price; if now above threshold, skip |

---

## Security Notices

- **Risk Level: Standard** — Each trade requires explicit user confirmation
- **Financial Risk**: You can lose your entire stake on any single round. 5-minute prediction markets are high-variance
- **Not Financial Advice**: This skill provides a momentum-based signal as a tool. It does not guarantee profits. Past momentum patterns do not predict future outcomes
- **No Private Keys**: This skill never requests private keys, seed phrases, or OTP codes
- **Strategy Attribution**: All trades are tagged with `--strategy-id pm-5m-momentum-sniper` for transparent tracking on the Plugin Store leaderboard

---

## Skill Routing

- For depositing funds to Polymarket → use `polymarket-plugin deposit`
- For withdrawing funds from Polymarket → use `polymarket-plugin withdraw`
- For non-5M prediction markets → use `polymarket-plugin list-markets`
- For basket/portfolio strategies → use `pmbasket`
- For manual direction picking → use `pm-5m-crypto-arena`

---

## Examples

### Example 1: Start a BTC Momentum Session

**User**: "Run momentum sniper on BTC, 10 USDC per round, 100 budget"

**Agent**:
1. Runs pre-flight checks ✅
2. Confirms: Coin=BTC, Stake=10, Budget=100, Max losses=3
3. Fetches `list-5m --coin BTC --count 10`
4. Last 3 resolved: Up, Up, Down → score +1 (Weak Up)
5. Active market Up price: 0.53
6. Presents: "📊 Round 1 | Signal: UP (+1) | Price: 0.53 | Stake: 10 USDC — Confirm?"
7. User: "yes"
8. Executes: `polymarket buy --market-id btc-5m-xxx --outcome up --amount 10 --strategy-id pm-5m-momentum-sniper --confirm`
9. Waits for resolution...
10. Reports result and moves to Round 2

### Example 2: Signal Check Only

**User**: "What's the ETH 5m signal?"

**Agent**:
```
🔍 ETH 5M Momentum Signal
Last 3 rounds: Down ❌ | Down ❌ | Down ❌
Momentum score: -3 (Strong Down)
Current Down price: 0.48
Verdict: ✅ Strong edge — Down at 0.48 is excellent value
```

### Example 3: Skip Round

**Agent** (during session):
```
⏭️ Round 5 skipped — no edge
Momentum score: 0 (neutral) | Up: 0.51 | Down: 0.49
Both sides near 50/50, no directional conviction. Waiting for next market.
```
