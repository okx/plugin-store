---
name: pmclock
description: "5-minute crypto Up/Down arbitrage scanner for Polymarket. Compares CEX-implied probability vs PM ask, deploys risk-bounded buys via polymarket-plugin when edge >= 1.5%."
version: "1.0.0"
author: "dddd86971-cloud"
tags:
  - polymarket
  - prediction-market
  - arbitrage
  - 5min
  - latency-arb
  - crypto
  - automated-trading
  - passive-income
  - agentic-wallet
  - strategy
---

# pmclock

## Overview

`pmclock` turns Polymarket's 5-minute crypto Up/Down markets into a
deterministic arbitrage scanner. It pulls live BTC/ETH/SOL spot prices
from a CEX (Binance public API), computes the GBM-implied probability
that the underlying closes above each market's strike at resolution time,
compares to the current PM YES/NO ask, and surfaces decisions only when
the **net edge ≥ 1.5%** (after expected fee + slippage).

What makes pmclock different from a hand-coded arb bot:

- The math (probability + Kelly-fractional sizing) is in a **compiled
  TypeScript binary** — same input → byte-identical output, across runs
  and models. Stable `planHash` SHA-256 fingerprint per scan.
- **LLM-Skill friendly cadence (~30s polling)** — pmclock is *not*
  competing with Rust-on-VPS HFT bots that dominate top-volume markets.
  It targets the **first 1-2 minutes after a 5-min market opens**, where
  thin liquidity ($50-200 books) lets a small AI-Skill operator capture
  edge that high-frequency bots ignore as too small to scale.
- **All on-chain writes go through `polymarket-plugin`** — pmclock never
  signs, never holds keys, never bypasses the Agentic Wallet TEE.

**Risk level: `advanced`.** Although liability per market is bounded
to the deployed amount (no leverage on prediction markets), the scanner
places multiple live orders per minute. Read **Security Notices** before
using with real funds.

## When to Use

Use this Skill when the user:

- has **USDC.e on Polygon** (Polymarket's collateral; not generic USDC)
  and wants AI-driven 5-min crypto arbitrage
- is comfortable with **automated decision-making** based on a public
  CEX price feed + Polymarket order book
- wants **deterministic, reproducible, audit-able** decisions
  (LLM agents can drift; pmclock binary cannot)
- is in a **non-restricted region** (US, France, Singapore, and a few
  others are blocked by Polymarket TOS — `polymarket-plugin check-access`
  enforces this in pre-flight)

Do **not** use this Skill when:

- the user is in a Polymarket-restricted jurisdiction
- the user expects guaranteed returns (this is statistical arbitrage —
  individual scans can lose; the edge is in long-run repetition)
- the user wants to bet on event direction (sports / elections /
  political markets) — pmclock is exclusively for crypto Up/Down 5-min
- the user wants to compete in top-of-book HFT (use a Rust + VPS bot
  instead — pmclock is intentionally slower and targets the long tail)

## Pre-flight Checks

Before using this Skill, the agent must verify:

1. The `onchainos` CLI is installed and the Agentic Wallet is unlocked.
2. **`polymarket-plugin` is installed.** If not, instruct the user:
   `npx skills add okx/plugin-store --skill polymarket-plugin`
3. The `pmclock` binary is on PATH (installed automatically when this
   Skill is installed via `npx skills add okx/plugin-store --skill
   pmclock`). Verify with `pmclock --version` (expect `1.0.0`+).
4. **Region check.** Run `polymarket-plugin check-access`. If the user
   is in a restricted region, refuse to proceed and explain why.
5. **Polygon wallet funded with USDC.e.** Run `polymarket-plugin
   balance`. POLY_PROXY mode strongly recommended (gasless trades).
   Need ≥ $20 USDC.e and a small POL balance for any one-time setup.
6. **No existing pmclock-tagged open orders.** Run `polymarket-plugin
   get-positions`; if any are tagged `pmclock` and approaching expiry,
   warn the user before spawning new ones.
7. **Dry-run mode is the default.** Every new session presents the
   `PmclockPlan` for user review BEFORE any `polymarket-plugin buy`
   call. Live execution requires explicit user confirmation
   ("go live", "place it", or equivalent).

## Attribution Rule

Every `buy`, `sell`, or `cancel` write operation routed to
`polymarket-plugin` MUST include `--strategy-id pmclock` for OKX
Plugin Store leaderboard attribution:

- Pass `pmclock` (10 chars) to the basic plugin's `--strategy-id` flag.
- Read-only operations (`list-5m`, `list-markets`, `get-market`,
  `get-positions`, `balance`) do not need the flag.

This tagging is what lets the Plugin Store Season 1 Challenge
leaderboard correctly attribute trades to this Skill. Untagged trades
are aggregated to the generic `polymarket-plugin` account and not
counted toward pmclock's leaderboard position.

## polymarket-plugin command map

This Skill is built against `polymarket-plugin` **v0.4.10** (pinned in
`dependent_plugin` in `plugin.yaml`). Concrete commands the agent calls:

| Semantic purpose | polymarket-plugin command |
|---|---|
| Region eligibility check | `polymarket-plugin check-access` |
| Wallet status / balances | `polymarket-plugin quickstart` / `balance` |
| Fund proxy wallet (one-time) | `polymarket-plugin setup-proxy` then `deposit --amount <usdc>` |
| List next 5-min Up/Down crypto markets | `polymarket-plugin list-5m --coin <BTC\|ETH\|SOL> --count <N>` |
| Get a specific market's order book | `polymarket-plugin get-market --market-id <id>` |
| Place a maker-style limit buy | `polymarket-plugin buy --market-id <id> --outcome <yes\|no> --amount <usdc> --price <0-1> --order-type GTC --post-only --expires <unix_seconds> --strategy-id pmclock --confirm` |
| Cancel an open order | `polymarket-plugin cancel --order-id <id> --confirm` |
| List user's open orders | `polymarket-plugin get-positions` |
| Redeem winning outcome tokens (after resolution) | `polymarket-plugin redeem --market-id <id> --confirm` |

**CEX price + recent realized vol** comes from a public-read source the
agent calls directly (declared in `plugin.yaml` `api_calls`):

```bash
# Spot price (live):
curl -s "https://api.binance.com/api/v3/ticker/price?symbol=BTCUSDT" \
  | jq -r '.price'

# Recent realized vol from 1-minute klines (last 60 minutes):
curl -s "https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1m&limit=60" \
  | jq -r '[.[][4] | tonumber] as $closes
           | reduce range(1; $closes|length) as $i (0;
               . + (($closes[$i]/$closes[$i-1] | log) as $r | $r*$r))
           | sqrt * sqrt(60 * 24)'   # → daily vol fraction
```

ETH/SOL: replace `BTCUSDT` with `ETHUSDT`/`SOLUSDT`.

## What's new in v1.0.0

Initial release. Features:

- **Deterministic 5-min arbitrage scanner** — Polymarket Up/Down
  markets vs CEX-implied GBM probability. Same input → same plan, byte
  for byte.
- **Standard normal CDF in pure TypeScript** (Abramowitz & Stegun
  7.1.26 approximation, max error 1.5e-7) — no external numerics
  library, no LLM math.
- **Kelly-fractional sizing** with three risk profiles (conservative
  0.15× Kelly / balanced 0.25× / aggressive 0.33×) and saturation
  ramp at 5% edge.
- **7 structured rejection reasons** — every skipped market gets a
  reason (`expired`, `too_far_out`, `low_liquidity`,
  `edge_below_threshold`, `yes_no_ask_invalid`, `insufficient_budget`,
  `unknown_direction`) so the user can audit "why didn't pmclock buy
  this one?".
- **Hard caps in source**: $200 total notional / $20 per market /
  $50 minimum book liquidity / 1.5% min edge / 0.5% absolute floor.
- **18 self-tests** covering CDF correctness, ATM/ITM/OTM probability
  bounds, sizing monotonicity, determinism, planHash stability,
  per-reason rejection, direction handling, greedy budget enforcement.

## Commands

### scan-5min

Run one arbitrage scan over the next batch of 5-minute Up/Down markets
for a chosen coin. **Does not place orders.**

**When to use:** User asks "scan BTC 5min for arb", "find me arb on
ETH", "is there edge right now". Or as the recurring loop body when
running pmclock on a timer.

**Agent execution steps:**

1. Run `polymarket-plugin check-access`. If region restricted → abort.

2. Fetch live CEX spot:
   ```bash
   PRICE=$(curl -s "https://api.binance.com/api/v3/ticker/price?symbol=${COIN}USDT" | jq -r '.price')
   ```

3. Fetch recent realized daily vol from 1m klines:
   ```bash
   VOL=$(curl -s "https://api.binance.com/api/v3/klines?symbol=${COIN}USDT&interval=1m&limit=60" \
     | jq -r '[.[][4] | tonumber] as $c
              | reduce range(1; $c|length) as $i (0;
                  . + (($c[$i]/$c[$i-1] | log) as $r | $r*$r))
              | sqrt * sqrt(60 * 24)')
   ```

4. Fetch current 5-min markets:
   ```bash
   polymarket-plugin list-5m --coin "$COIN" --count 5
   ```
   For each returned market, also call `get-market --market-id <id>` to
   pull top-of-book yesAsk / noAsk / liquidity.

5. Pipe everything into `pmclock plan`:
   ```bash
   echo '{
     "coin": "BTC",
     "cexPrice": '$PRICE',
     "cexVolDaily": '$VOL',
     "pmMarkets": [
       { "marketId":"<id>", "threshold":<strike>, "direction":"above|below",
         "closesAt":<unix_ms>, "yesAsk":<0-1>, "noAsk":<0-1>,
         "yesLiquidityUsd":<num>, "noLiquidityUsd":<num> }
     ],
     "totalNotionalUsd": 50,
     "maxPerMarketUsd": 15,
     "edgeThresholdPct": 0.015,
     "riskProfile": "balanced",
     "nowMs": '$(($(date +%s)*1000))'
   }' | pmclock plan
   ```

6. Present the resulting `PmclockPlan` to the user as **DRY-RUN PLAN —
   no orders placed**. Include:
   - Number of decisions vs. markets scanned
   - Per-decision: side, amount, limit price, edge %, and rationale
   - Aggregated rejected-by-reason summary
   - planHash (first 6 chars) — same scan is reproducible
   - Any warnings

7. End with: "No orders have been placed. Reply **'place it'** to
   execute, or **'tighter'** to raise edge threshold to 2.5%."

**Tip:** For a human-readable summary, `pmclock explain --input
plan.json` formats the same data as plain English.

### scan-and-execute

Run a scan AND immediately execute decisions if `live-mode` is on.
**This is the only path that places real orders.**

**When to use:** Only after `scan-5min` has been shown to the user in
the same session and the user has explicitly confirmed.

**Mandatory pre-execution checks:**

1. `polymarket-plugin check-access` passes.
2. The plan was produced by `pmclock plan` in this session — never
   hand-constructed.
3. POLY_PROXY mode is set (or user has approved EOA gas costs per
   trade) — verify with `polymarket-plugin balance`.
4. If `plan.warnings` mentions `clamped`, surface it verbatim and ask
   for explicit acknowledgment.

**Execution loop:**

For each decision in `plan.decisions`:

```bash
polymarket-plugin buy \
  --market-id <decision.marketId> \
  --outcome <decision.side> \
  --amount <decision.amountUsd> \
  --price <decision.limitPrice> \
  --order-type GTC \
  --post-only \
  --expires <decision.expiresAtMs / 1000> \
  --strategy-id pmclock \
  --confirm
```

- `--post-only` — placement gets maker rebate; if the order would
  cross, polymarket-plugin returns an error rather than fill as taker.
- `--expires` — the limit naturally expires 30s before market
  resolution; no manual cancel needed.
- `--strategy-id pmclock` — leaderboard attribution.

If a single buy is rejected (insufficient balance, price band, rate
limit), stop immediately, quote the exact error, and ask the user how
to proceed. Never silently retry, never silently skip.

**After resolution (≥ 5 min later):** Run `polymarket-plugin redeem
--market-id <id> --confirm` for any winning outcomes the user holds.

### scan-loop

Recurring scan-and-execute on a 30-60s cadence for a fixed total
duration or budget.

**When to use:** User says "run pmclock for 1 hour with $50 budget" or
"keep arbing while I grab dinner".

**Agent execution steps:**

1. Confirm explicit time and budget bounds with the user.
2. Loop: `scan-5min` → if any decision and live-mode → `scan-and-execute`
   → sleep 30-60s → repeat.
3. After each iteration, show running total: deployed, fills, redeemed.
4. Stop conditions (any of):
   - Time bound reached
   - Cumulative deployed ≥ session budget
   - Cumulative realized PnL < −5% of session budget (loss-stop)
   - User says "stop"
5. End with a session summary + redemption queue (winning markets that
   need a `redeem` call after resolution).

### caps

Emit the hard caps as JSON for the agent to surface in the UI.

```bash
pmclock caps
```

## Examples

### Example 1 — single BTC scan, dry-run

**User:** "Scan BTC 5min arb for me, $50 budget, balanced"

**Agent:** runs the `scan-5min` flow above. Returns:

```
Plan a3f82b  (BTC, balanced)

• CEX price       : $77,500
• CEX vol (daily) : 1.74%
• Markets scanned : 5
• Edge threshold  : 1.50%
• Budget          : $50, deployed $7.50

Decisions (1):
  NO   $  7.50 @ 0.610  edge 38.90%  (CEX 99.9% vs PM 61.0%)
        btc-above-78000-15-30

Rejected (4):
  3 × edge_below_threshold
  1 × low_liquidity
```

> No orders placed. Reply 'place it' to execute, 'tighter' to raise to 2.5%.

### Example 2 — region-restricted user

**User (US):** "Run pmclock"

**Agent:** runs `polymarket-plugin check-access` first. Sees blocked.
Refuses with a clear message: "Polymarket's terms of service block
trading from US IP addresses. pmclock cannot proceed. If you're
travelling to a non-restricted region, run `check-access` again."

### Example 3 — too-thin liquidity day

**User:** "Run pmclock for an hour"

**Agent:** runs `scan-loop`. After 10 iterations, every market is
rejected for `low_liquidity`. The agent surfaces this and suggests
either:
- waiting for a more active session (US trading hours have deeper books)
- lowering `MIN_LIQUIDITY_USD` (would require a code change + version
  bump — not user-overridable).

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| `polymarket-plugin: command not found` | Basic plugin not installed | `npx skills add okx/plugin-store --skill polymarket-plugin` |
| `pmclock: command not found` | Binary not on PATH | Re-install via `npx skills add okx/plugin-store --skill pmclock`; verify `pmclock --version` |
| `pmclock error: plan input is not valid JSON` | Malformed input | Show the binary's error verbatim; ask agent to recheck the JSON construction |
| `polymarket-plugin check-access` fails (restricted) | User in blocked region | Refuse to proceed; tell the user the restriction is platform-side, not pmclock-side |
| Plan has 0 decisions, all `edge_below_threshold` | Markets are fairly priced right now | Normal during low-volatility periods; suggest waiting or trying ETH/SOL instead of BTC |
| Plan has many `low_liquidity` rejects | Off-hours / thin books | Suggest waiting for active hours (US 9am-4pm ET typically deepest) |
| Plan has `clamped` warning on totalNotional | User asked for > $200 | pmclock caps total at $200/scan — surface warning, confirm user wants to proceed |
| `polymarket-plugin buy` returns "insufficient balance" | Proxy wallet under-funded | `polymarket-plugin deposit --amount <N>`; never silently reduce decision sizes |
| `polymarket-plugin buy` returns "price band" / "rate limit" | Order rejected upstream | Quote exact error; stop; ask user how to proceed |
| Decision price differs from current ask by > 0.05 | Book moved during preview | Re-run `scan-5min` — pmclock plans are fresh-snapshot only |

## Security Notices

**Risk level: advanced.** This Skill places live limit orders on
Polymarket. Each market is bounded-loss by design (a $5 buy can lose
at most $5), but multiple concurrent orders can compound. The
arbitrage edge depends on CEX price feeds being accurate — feed
manipulation or stale data can flip the sign of a "winning" bet.

**Hard limits enforced in `pmclock` source (`src/types.ts:CAPS`):**

- Maximum **$200** total notional per scan.
- Maximum **$20** per individual market.
- Minimum **$50** combined liquidity (YES + NO depth) to consider
  a market.
- Minimum **1.5%** net edge to fire a decision (default; floor at
  0.5% if user lowers).
- Minimum **30s** time-to-close (skip markets too close to resolution).
- Maximum **600s** time-to-close (skip markets > 10 min out).

These limits are baked into the binary source. They cannot be
overridden from within this Skill. To trade larger size, run multiple
sequential scans — each scan re-evaluates the live book.

**Behavioural safety:**

- **Dry-run by default.** Every scan presents the `PmclockPlan` for
  user review before any `polymarket-plugin buy` call.
- **Explicit confirmation** required to execute (`'place it'` or
  equivalent).
- **Deterministic decisions.** Math is in a compiled binary, not the
  LLM. Same scan inputs → same decisions, byte-for-byte.

**What this Skill does not do:**

- It does not hold or touch private keys. All signing happens inside
  the Agentic Wallet TEE via `polymarket-plugin`.
- It does not collect, store, or transmit user wallet addresses,
  balances, or trade history to any external server outside the
  declared `api_calls` (Binance public ticker for spot prices,
  Polymarket gamma API for markets — both read-only public data).
- It does not run ML or use any predictive model beyond a vanilla
  GBM probability calculation. There is no "AI black box" — the math
  is fully spelled out in `src/arb.ts`.

**Disclaimer:** Statistical arbitrage on prediction markets has
non-zero risk: the CEX price feed may be stale or manipulated; the PM
order book may move adversely between scan and execution; resolution
may be subject to oracle disputes. Nothing in this Skill is financial
advice. The user is responsible for their own sizing, risk management,
and compliance with local regulations.

## Skill Routing

- **For non-arbitrage Polymarket buys** (event basket, single-market
  directional bet) → use `polymarket-plugin` directly with manual
  market selection.
- **For Hyperliquid perp grids** → use the `hyperliquid-aigrid` Skill.
- **For wallet / portfolio overview** → use OKX's portfolio Skills.
