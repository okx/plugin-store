---
name: hyperliquid-funding-rate-scout
description: "Identify high-probability mean reversion opportunities by scanning Hyperliquid perpetual futures for statistically significant funding rate imbalances and crowded positioning"
version: "1.0.0"
author: "Ritesh"
tags:
  - hyperliquid
  - funding-rate
  - mean-reversion
  - strategy
  - perpetuals
  - signal
  - analytics
---

# Hyperliquid Funding Rate Scout

## Overview

This is a **strategy skill** that enables AI agents to identify high-probability
mean reversion opportunities by scanning perpetual futures markets on Hyperliquid
for extreme funding rate imbalances. It operates as both a signal intelligence
layer and a conditional execution layer — scanning and analyzing autonomously,
then optionally triggering trade execution via `hyperliquid-plugin` only after
explicit user confirmation.

> 🔒 **Scanning and analysis are strictly read-only. This plugin does not execute
> trades, move funds, or interact with user wallets in any way without explicit
> user confirmation.**

Unlike generic funding rate trackers, this skill focuses specifically on
**statistically significant outliers** — filtering out normal market noise and
surfacing only the crowded positioning scenarios where traders are heavily biased
in one direction. These conditions historically precede mean reversion: funding
rates normalize, and positions aligned with the overcrowded side get squeezed.

Hyperliquid is used as the data source due to its transparent on-chain orderbook
and reliable real-time funding data, making it ideal for detecting short-term
market inefficiencies. Data is fetched in real-time at request, with lightweight
caching to ensure responsiveness without stale signals.

This plugin is designed as a signal layer within a modular trading agent system,
separating market intelligence from execution for safer and more controlled
workflows. After generating signals, it presents structured output and stops —
handing off only after explicit user confirmation.

---

## Trigger Conditions

Invoke this skill when:
- The user requests trading opportunities or market setups
- The user asks about funding rates, market sentiment, or positioning
- The user is looking for arbitrage, mean reversion, or crowding-based strategies
- The user mentions Hyperliquid, perps, or funding in any context

---

## Risk Disclaimer

> **NOTICE**: This plugin operates as a strategy skill with two distinct modes:
> read-only signal scanning (default) and optional trade execution via
> `hyperliquid-plugin` (user-triggered only). In execution mode, this strategy
> may place leveraged perpetual futures trades on Hyperliquid on the user's behalf.
> Leveraged trading carries significant risk including partial or total loss of
> funds and liquidation. All signals are generated from funding rate analysis and
> are strictly informational — not financial advice. Funding rate mean reversion
> is not guaranteed. Markets can remain in extreme funding conditions for extended
> periods. Validate all signals independently before acting. Trade sizing, risk
> management, and all execution decisions are entirely the user's responsibility.
>
> **Note**: Funding rate signals indicate crowd positioning, not guaranteed price
> reversal. Signals should be used alongside proper risk management.

---

## Pre-flight Checks

Before using this skill, ensure the following are installed and accessible:

1. Install the `onchainos` CLI:
   ```bash
   npx skills add okx/onchainos-skills
   export PATH="$HOME/.local/bin:$PATH"
   ```

2. Verify `onchainos` is working:
   ```bash
   onchainos --version
   ```

3. Install the Hyperliquid Plugin (needed only if user confirms execution later):
   ```bash
   npx skills add okx/plugin-store --skill hyperliquid-plugin
   ```

4. The signal scanner binary is included with this skill and requires no separate
   installation. It uses `curl` for HTTP requests — verify curl is available:
   ```bash
   curl --version
   ```

5. To run the scanner directly (outside of agent context):
   ```bash
   # Default scan — top 5 signals above 0.01% hourly threshold
   hyperliquid-funding-rate-scout

   # Custom threshold and limit
   hyperliquid-funding-rate-scout --threshold 0.03 --limit 3

   # Filter to specific assets
   hyperliquid-funding-rate-scout --asset BTC --asset ETH

   # Raw JSON output (for programmatic use)
   hyperliquid-funding-rate-scout --json
   ```

> **Note**: No wallet connection is required for this plugin. All scan and
> analysis operations are strictly read-only.

---

## Inputs

The agent should check for optional user-provided parameters before scanning.
If not provided, use the defaults listed below.

| Input | Type | Default | Description |
|-------|------|---------|-------------|
| `asset_filter` | string or list | all markets | Limit scan to specific asset(s), e.g. BTC, ETH, SOL |
| `funding_threshold` | number (APR %) | ±50% | Minimum absolute funding rate to flag as a signal |
| `num_signals` | integer | 5 | Number of top signals to return |

**How to collect inputs from the user:**

If the user gives a broad request (e.g. "find me opportunities"), use all defaults.
If the user specifies constraints (e.g. "only BTC and ETH" or "show top 3 above 80% APR"),
extract those values and apply them to the scan.

---

## Commands

### Phase 1 — Scan: Fetch All Funding Rates

```bash
onchainos signal list --chain arbitrum --platform hyperliquid
```

**When to use**: Always — this is the entry point of every scan session.

**What it returns**: All active Hyperliquid perpetual markets with their current
funding rates. Each entry includes the asset symbol, funding rate (hourly and
annualized), and directional bias.

**If asset_filter is set**, cross-reference output and retain only the
specified assets before proceeding to Phase 2.

---

### Phase 1b — Get Price and Market Context per Asset

For each asset flagged in Phase 1, fetch current market data:

```bash
onchainos market price --address <ASSET_SYMBOL> --chain arbitrum
```

**When to use**: After the scan, for every asset that passes the threshold filter.
This provides the current price, 24h change, and open interest context needed
to build the signal card.

**Example**:
```bash
onchainos market price --address BTC --chain arbitrum
onchainos market price --address SOL --chain arbitrum
```

---

### Phase 2 — Analyze: Get Smart Money Positioning

Cross-reference funding extremes with on-chain smart money signals to assess
whether the crowding thesis is supported by institutional positioning data.

```bash
onchainos signal list --chain arbitrum
```

**When to use**: After filtering by funding threshold, run this to check
smart money direction. If whale/smart money positioning is **opposite** to the
crowded side, the mean reversion thesis is strengthened. If aligned with the
crowd, note it as a risk factor.

---

### Phase 2b — Analyze: Get Recent Price History

Fetch recent kline data to assess whether the market is trending or ranging.

```bash
onchainos market kline --address <ASSET_SYMBOL> --chain arbitrum --interval 1h
```

**When to use**: For the top 1–3 signals. Mean reversion strategies have
higher probability in ranging markets. A strong directional trend is a risk
factor that must be noted in the signal card.

---

## Agent Execution Flow

The agent follows three strict phases. **Phase 3 ends with a full stop — no
execution occurs without explicit user confirmation.**

---

### PHASE 1 — SCAN

```
1. Check for optional inputs: asset_filter, funding_threshold, num_signals
2. Run: onchainos signal list --chain arbitrum --platform hyperliquid
3. Parse all funding rates from output
4. If asset_filter is set → retain only matching assets
5. Apply threshold filter: keep only markets where |funding rate APR| > funding_threshold
6. If no markets pass the filter → go to "No Signals" response (see below)
7. Sort remaining markets by |funding rate| descending
8. Retain top N markets where N = num_signals
9. For each retained market: run onchainos market price to fetch price + OI context
```

---

### PHASE 2 — ANALYZE

```
For each signal candidate (top N assets from Phase 1):

10. Classify direction:
    - funding rate > 0 → longs overcrowded → SHORT signal (mean reversion: shorts collect funding)
    - funding rate < 0 → shorts overcrowded → LONG signal  (mean reversion: longs collect funding)

11. Assess crowding severity using hourly funding rate (not APR):
    - hourly rate > 0.01% → **Elevated** (moderate signal, worth monitoring)
    - hourly rate > 0.03% → **High** (strong signal, crowding is significant)
    - hourly rate > 0.05% → **Extreme** (highest conviction, historically unsustainable)

12. Run smart money check (onchainos signal list --chain arbitrum)
    - Smart money opposite to crowd → Bullish confirmation, note as supporting factor
    - Smart money aligned with crowd → Note as risk factor

13. Run kline check for top 1–3 signals
    - Ranging / consolidating price action → Favorable for mean reversion
    - Strong directional trend → Note as risk factor, lower conviction rating

14. Build signal card for each (see output format below)
```

---

### PHASE 3 — PRESENT AND STOP

```
15. Present all signal cards to user (ranked #1 to #N)
16. Add a summary line: total markets scanned, signals found, threshold used
17. Include timing note: minutes until next 8h settlement window (if within 90 min, highlight urgently)
18. Ask: "Would you like to act on any of these signals?"
19. *** STOP: DO NOT EXECUTE WITHOUT USER CONFIRMATION ***

If user says YES to a signal:
20. Collect: asset, position side, size (USDC), leverage, stop-loss price
21. Present trade summary for user review
22. Ask: "Confirm execution?" — wait for explicit approval
23. Only after confirmed → hand off to hyperliquid-plugin (see Strategy Execution Mode)
24. After execution completes → immediately offer: "Scan for more opportunities?" to
    encourage re-engagement and help users capture multiple signals per session.
```

---

## Signal Card Output Format

Present each signal in this structured format. Keep language clear and
non-technical enough for any trader to understand.

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📡 SIGNAL #<N> — <ASSET>/USD-PERP
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Direction:      <LONG / SHORT>
Signal Type:    Mean Reversion — Funding Rate Extreme
Conviction:     <Elevated / High / Extreme>

Funding Rate:   <+XX.XX% / -XX.XX%> APR  (hourly: <±X.XXXX%>)
Current Price:  $<XX,XXX>
24h Change:     <+X.X% / -X.X%>

📌 Thesis
<2–3 sentences explaining why this is an opportunity.
Example: "BTC perp funding is at +187% APR, meaning longs are paying shorts
at nearly 0.51% every 8 hours. This level of crowding is historically
unsustainable and typically precedes a funding rate normalization. A short
position here collects funding while waiting for the imbalance to unwind.">

✅ Supporting Factors
- <e.g. Smart money positioned short — aligned with mean reversion thesis>
- <e.g. Price in consolidation range for 18h — favorable for reversion>

⚠️  Risk Factors
- <e.g. BTC in strong uptrend — momentum may delay reversion>
- <e.g. High open interest — potential for volatile squeeze>

Entry Context:  ~$<price> (current market)
Watch Level:    <Key price level to monitor, e.g. recent high/low>
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## No Signals Response

When no markets pass the threshold filter, respond with:

```
No statistically significant funding rate imbalances detected above the
±<threshold>% APR threshold at this time.

Markets scanned: <N>
Signals found:   0
Threshold used:  ±<X>% APR

This suggests Hyperliquid perp markets are currently near equilibrium.
Funding rates tend to spike most around the 8-hour settlement windows
(00:00, 08:00, 16:00 UTC). Consider checking back then, or lower the
threshold to see moderate-level signals.
```

---

## Strategy Execution Mode

This section is only reached after the user has explicitly confirmed a trade.
Read-only scanning is always the default behavior. Execution is strictly
optional and user-triggered — the agent must never initiate execution
autonomously under any circumstances.

**Behavior rules:**
- Scanning and signal generation: always autonomous, always read-only
- Execution: optional, requires hard confirmation from the user
- NEVER proceed to execution based on inference, assumption, or partial confirmation
- NEVER re-trigger a previously confirmed trade without a new explicit confirmation

Once the user confirms, pass control to `hyperliquid-plugin` with the confirmed
parameters and the required `--strategy-id` attribution flag:

```bash
# Short execution
hyperliquid-plugin place-order \
  --side short \
  --asset <SYMBOL> \
  --size <USDC_AMOUNT> \
  --leverage <LEVERAGE> \
  --stop-loss <STOP_PRICE> \
  --strategy-id hyperliquid-funding-rate-scout \
  --confirm

# Long execution
hyperliquid-plugin place-order \
  --side long \
  --asset <SYMBOL> \
  --size <USDC_AMOUNT> \
  --leverage <LEVERAGE> \
  --stop-loss <STOP_PRICE> \
  --strategy-id hyperliquid-funding-rate-scout \
  --confirm
```

> **Note**: `--strategy-id hyperliquid-funding-rate-scout` must be included on
> every execution call for leaderboard attribution. Never omit this flag.

Before calling the execution plugin, always present this confirmation block:

```
📋 TRADE CONFIRMATION
Asset:       <SYMBOL>/USD-PERP
Side:        <LONG / SHORT>
Size:        <USDC_AMOUNT> USDC
Leverage:    <X>x
Stop-Loss:   $<PRICE>
Est. Liquidation: $<calculated price>
Strategy ID: hyperliquid-funding-rate-scout

Proceed with execution? (yes / no)
```

Only call `hyperliquid-plugin` after receiving an explicit "yes".

---

## Timing Intelligence

Include this context when presenting signals, especially for high-conviction ones:

- Hyperliquid funding settles every **8 hours**: 00:00, 08:00, 16:00 UTC
- Funding rates tend to spike or accelerate in the **60–90 minutes before settlement**
- The highest-conviction scan window is **30–60 minutes before each settlement**
- Post-settlement, rates often reset — a new scan is recommended after each window

**Urgency rules for the agent:**
- If current time is within 90 minutes of a settlement window → prepend signals with:
  `⏰ Settlement in ~<X> minutes — funding rates are near peak. High-conviction window.`
- If current time is within 30 minutes → prepend with:
  `🔴 Settlement in <X> minutes — act now or wait for next window after rates reset.`
- After a settlement passes → proactively suggest:
  `Funding rates just settled. This is a good time to scan for new imbalances building up.`

If the user's current time is near a settlement window, proactively mention it.

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| No markets returned from scan | API unavailable or network issue | Wait 10 seconds, retry scan |
| `Platform not found` | Hyperliquid flag not supported | Remove `--platform hyperliquid` and filter manually |
| All rates in neutral zone | Markets at equilibrium | Inform user, suggest lower threshold or retry near settlement window |
| Asset not found in price fetch | Delisted or low-liquidity market | Skip price fetch for that asset, note in signal card |
| Rate limit error | Too many requests in short window | Wait 15 seconds before retrying |
| kline data unavailable | New listing or no history | Skip kline analysis, note absence in signal card |
| Hyperliquid Plugin not installed | Only needed at execution stage | Run `npx skills add okx/plugin-store --skill hyperliquid-plugin` |

---

## Security Notices

- **Risk Level**: STANDARD — read-only by default; optional execution mode places leveraged perpetual trades
- Scanning and analysis require no wallet connection and access no private keys
- Execution only occurs via `hyperliquid-plugin` after explicit user confirmation
- All execution calls include `--strategy-id hyperliquid-funding-rate-scout` for leaderboard attribution
- All on-chain writes (if triggered) are handled securely by Onchain OS
- This plugin does not store, log, or transmit any user data or wallet addresses

---

## Skill Routing

- To execute a confirmed trade → `hyperliquid-plugin`
- To DCA into an identified position → `hyperliquid-dca-bot`
- To check wallet balance before execution → `onchainos wallet balance --chain arbitrum`
- To verify token contract security → `okx-security` skill
- To view current portfolio and PnL → `okx-wallet-portfolio` skill
- For prediction market opportunities → Polymarket plugin