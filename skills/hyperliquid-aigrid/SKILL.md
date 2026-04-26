---
name: hyperliquid-aigrid
description: "AI-driven Hyperliquid grids. Deterministic binary, funding-aware sizing (±20% tilt), concentrated liquidity, 75-combo parameter optimizer, 30-day backtest. One sentence in, risk-capped plan out."
version: "1.2.6"
author: "dddd86971-cloud"
tags:
  - hyperliquid
  - grid
  - perpetuals
  - strategy
  - agentic-wallet
  - passive-income
  - automated-trading
  - funding-aware
  - backtest
  - range-bound
---

# hyperliquid-aigrid

## Overview

`hyperliquid-aigrid` turns a trader's one-sentence view on a Hyperliquid perpetual —
"BTC will chop between 90k and 95k, aggressive grid, $3000 at 5x" — into a
fully specified, risk-capped grid strategy.

What makes hyperliquid-aigrid different from plain Skill-based grids:

- A **deterministic TypeScript engine** (the `hyperliquid-aigrid` binary, shipped with
  this Skill) computes the grid plan. Same inputs → same output, every
  time, across runs and models.
- The LLM agent handles **natural-language parsing** and **live market
  data fetching** (through the Hyperliquid basic plugin). The engine
  handles **math**. Clear separation of responsibilities.
- All on-chain writes go through the **Hyperliquid basic plugin**, inside
  the Agentic Wallet. hyperliquid-aigrid never bypasses it.

**Risk level: `advanced`.** This Skill places and manages multiple live
perpetual orders. Read Security Notices before using it with real funds.

## When to Use

Use this Skill when the user:

- has a **range-bound view** on a liquid Hyperliquid perpetual — i.e.
  expects the price to oscillate inside a band over hours or days,
  rather than trending strongly
- wants **passive execution** (limit orders working at pre-computed
  levels) rather than active discretionary entries
- wants **predictable, deterministic** grid parameters rather than
  LLM-guessed levels
- is comfortable with multiple simultaneous open orders and has enough
  collateral for the chosen notional and leverage

Do **not** use this Skill when:

- the user describes a directional thesis ("BTC will break out to
  100k") — a grid will accumulate a losing position against a trend;
  direct them to a directional Skill instead
- the market is in a clear strong trend — the Skill should refuse and
  explain why, not silently build a plan that will bleed
- the user wants leverage above 10× or notional above $5,000 — those
  are hard caps, not soft preferences

## Profile Selection Rules

The three risk profiles control grid density and stop-loss width.
When the user doesn't specify one, choose using these rules:

Default to `conservative` when:

- the user is new to hyperliquid-aigrid (first session, no prior grids)
- the user expresses caution, mentions "small", "safe", or doesn't
  specify a notional larger than $500
- the user chose leverage ≥ 5× (compound aggressive-leverage and
  aggressive-density = unnecessary risk)
- the market's recent realized volatility is unusually high (>5% daily)

Use `balanced` when:

- the user has run hyperliquid-aigrid before without issue
- the user specifies "normal", "balanced", "medium", or doesn't signal
  caution
- leverage ≤ 5× and notional is in the $500–$2,000 range

Use `aggressive` when:

- the user explicitly says "aggressive", "max out", "as many fills as
  possible", or asks for a dense grid
- the user has previously used `balanced` successfully
- the range is wide (>15% of mid-price) — a dense grid has room to
  breathe without crowding ticks

These are defaults. The user can always override by stating the profile
directly.

## First Session Flow

When this Skill has never been used by the current user (no prior
hyperliquid-aigrid-tagged fills in account history), prefer this introduction:

1. Start in **dry-run** with `conservative` profile regardless of what
   the user typed, unless they explicitly demanded otherwise.
2. Build a grid with ≤ $200 notional and ≤ 2× leverage even if the user
   asked for more — frame it as a "let's see how it looks first" plan.
3. Present the dry-run plan with the `planHash` prefix, the stop-loss,
   the expected fills/day, and the approximate liquidation buffer.
4. Explicitly tell the user: "This is a preview. Nothing is placed. If
   you're comfortable, say 'go live' and I'll place it. If you want the
   larger plan you originally described, say 'larger' and I'll rebuild."
5. Only after the user has successfully completed one `conservative`
   grid (opened and closed at least one), follow their requested
   profile and size without this training-wheels adjustment.

## Pre-flight Checks

Before using this Skill, the agent must verify:

1. The `onchainos` CLI is installed and the Agentic Wallet is unlocked.
2. The **Hyperliquid basic plugin** is installed. If not, instruct the user:
   `npx skills add okx/plugin-store --skill hyperliquid-plugin`
3. The `hyperliquid-aigrid` binary is available on PATH (installed automatically when
   this Skill is installed via `npx skills add okx/plugin-store --skill
   hyperliquid-aigrid`). Verify with `hyperliquid-aigrid --version`.
4. The user has funded their Hyperliquid perp account with at least the
   minimum collateral for the chosen instrument (typically ≥ 20 USDC).
5. **No existing active hyperliquid-aigrid grid on the same instrument.** Call the
   Hyperliquid basic plugin to check open orders; if any orders tagged
   as hyperliquid-aigrid (see Attribution Rule below) are open on the requested
   instrument, refuse to open a second grid there unless the user
   explicitly says "replace the existing grid". Stacking multiple
   hyperliquid-aigrid grids on the same instrument can produce offsetting orders
   and unpredictable P&L.
6. **Circuit-breaker state is clear.** Call the Hyperliquid basic plugin
   to fetch the user's fills from the last 24 hours. If cumulative
   realized PnL from hyperliquid-aigrid-tagged trades is worse than −2% of
   account equity, refuse to open a new grid until the next UTC day.
   Tell the user why and when they can retry.
7. **Dry-run mode is the default.** Every new session starts in dry-run.
   Live execution requires an explicit user confirmation in natural
   language ("go live", "place real orders", etc.).

## Attribution Rule

Every write operation routed to the Hyperliquid basic plugin must include
the strategy tag `hyperliquid-aigrid` in whatever tag/cloid/strategy-id field the
basic plugin exposes:

- If the basic plugin accepts a `strategy_id`, `tag`, or `clientOrderId`
  parameter, pass `hyperliquid-aigrid` (or `ha-{planHash[:6]}` for per-plan
  granularity).
- If the basic plugin exposes a `--strategy-id` CLI flag, append
  `--strategy-id hyperliquid-aigrid`.
- If no such field exists, embed `hyperliquid-aigrid` in any free-form order-label
  field available.

Read-only market inspection (getting mark price, candles, meta) does not
need tagging.

This tagging is what lets the Plugin Store Season 1 Challenge leaderboard
correctly attribute trades to this Skill. Untagged trades may be
aggregated to the generic `hyperliquid-plugin` and not counted toward
hyperliquid-aigrid's leaderboard position.

## Hyperliquid basic-plugin command map

This Skill is built against `hyperliquid-plugin` **v0.3.9** (the version
pinned in `dependent_plugin` in `plugin.yaml`). Concrete commands the agent
calls:

| Semantic purpose | hyperliquid-plugin command |
|---|---|
| Get current mid price for a coin | `hyperliquid-plugin prices --coin <COIN>` |
| Place a perp limit / market order | `hyperliquid-plugin order --coin <COIN> --side buy\|sell --size <COIN_QTY> --price <PRICE> --order-type limit\|market --strategy-id hyperliquid-aigrid --confirm` |
| Place a stop-loss bracket on a position | `hyperliquid-plugin tpsl --coin <COIN> --sl-px <PRICE> --strategy-id hyperliquid-aigrid --confirm` |
| Close a position at market | `hyperliquid-plugin close --coin <COIN> --strategy-id hyperliquid-aigrid --confirm` |
| Cancel a single resting order | `hyperliquid-plugin cancel --order-id <OID> --confirm` |
| Cancel many in one signed request | `hyperliquid-plugin cancel-batch --order-ids <OID,OID,...> --confirm` |
| List open orders for the wallet | `hyperliquid-plugin orders` |
| List open positions + margin summary | `hyperliquid-plugin positions` |
| Place many limit orders in one signed batch | `hyperliquid-plugin order-batch --orders <FILE> --strategy-id hyperliquid-aigrid --confirm` |

**Funding rate (used for v1.1 funding-aware bias)** is **not** exposed by
hyperliquid-plugin v0.3.9. The agent reads it directly from Hyperliquid's
public read-only info endpoint (declared in `plugin.yaml` `api_calls`):

```bash
# Replace BTC with the target coin (case-sensitive, matches universe[].name)
COIN=BTC
curl -s -X POST https://api.hyperliquid.xyz/info \
  -H 'Content-Type: application/json' \
  -d '{"type":"metaAndAssetCtxs"}' \
  | jq -r --arg coin "$COIN" '
      (.[0].universe | map(.name) | index($coin)) as $i
      | if $i == null then "null" else (.[1][$i].funding) end
    '
# Output is the hourly funding rate as a string (e.g. "0.000019")
# or "null" if the coin is not found.
```

Pass that hourly funding rate (a fraction, e.g. `0.000019`) into the binary
as `marketMeta.fundingRateHourly`. If the call fails or returns "null", omit
the field — hyperliquid-aigrid falls back to a symmetric grid.

> **Mainnet only.** This Skill targets Hyperliquid mainnet. The testnet
> endpoint (`api.hyperliquid-testnet.xyz`) is intentionally *not* declared
> in `api_calls` because testnet funding is synthetic and can't be used
> as alpha. For dry-run testing without real money, use the binary's
> built-in `backtest` subcommand on real mainnet candles instead.

Under no circumstances should the agent call Hyperliquid for **writes** via
HTTP / RPC / any non-plugin path — every order must go through
`hyperliquid-plugin` so it goes through the Agentic Wallet TEE and carries
`--strategy-id hyperliquid-aigrid` for leaderboard attribution.

## What's new in v1.2.6

- **`explain` finally surfaces all the v1.2.2 / v1.2.4 fee + notional
  fields in human form** — pre-v1.2.6 the binary computed them but
  `explain` showed only the basic plan; users had to read raw JSON.
  New sections in `explain`:
  - `Notional split` — buy/sell aggregate + ratio + funding-tilt label
  - `Fee economics` — rung gap with safety margin (× break-even),
    expected fee per RT, net edge per RT, estimated daily net
- **`runOptimize` now ranks by net (post-fee) score, not gross.**
  Pre-v1.2.6 a tight grid that grossed $2 but burned $1.50 in fees
  could top the leaderboard. v1.2.6 uses
  `realizedPnlNetUsd / max(maxDD, 1)` so fee drag is part of the
  ranking, not papered over.
- **`BacktestResult.feesPaidUsd` + `realizedPnlNetUsd`** added —
  every simulated fill accumulates a maker fee, reported alongside
  the original (now-explicitly-gross) `realizedPnlUsd`.
- **`OptimizeCandidate` exposes the same breakdown** — users can
  audit "why does candidate A rank lower than B if A has higher
  gross?" right from the JSON.
- 49 self-tests (was 45 in v1.2.5).

`computeGridPlan()` and `quickstart()` semantics unchanged.
`planHash` byte-stable for the same input across v1.2.5 → v1.2.6.

## What's new in v1.2.5

- **Two new fine-grained user commands** documented in SKILL.md:
  - **`grid-cancel-rung`** — cancel ONE specific rung from an active
    grid, leaving the others intact. Resolves natural-language
    references like "撤掉最下面那个 buy" / "cancel the $77,608 rung" /
    "drop the closest sell" to the right oid via a documented
    disambiguation table. If ambiguous, lists candidates and asks.
  - **`grid-roll`** — atomically re-center an active grid around the
    current mark. Cancels old rungs, recomputes range from current
    markPrice + fresh candles (using v1.2.3's notional-aware
    geometry), places new rungs. Existing position is preserved by
    default; user can opt to flatten first ("roll + flatten").
- **`grid-roll` safety rules** — refuses if the existing position
  would be liquidated by the new range, or if any old rungs failed
  to cancel before new ones are placed. Never lets state get
  inconsistent (no double-deployment, no orphaned rungs).
- **No binary changes.** v1.2.5 is a Skill-orchestration layer release
  — `plan()` / `quickstart()` / `runBacktest()` / `runOptimize()` are
  byte-identical to v1.2.4. Documentation-only.
- 45 self-tests (unchanged from v1.2.4).

**Why these matter:** before v1.2.5, users who wanted to adjust an
active grid had to manually orchestrate `grid-close` → `grid-plan` →
`grid-open` (three commands, three confirmations, error-prone). The
new commands make "撤一根" and "整个挪" first-class operations with
one confirmation each.

## What's new in v1.2.4

- **Fee-aware plan output** — discovered during live $24-account testing
  on PR #360: maker fills carry a 1.5 bps fee on Hyperliquid tier-0,
  even with `crossed: false`. Pre-v1.2.4 the plan reported
  `expectedFillsPerDay` but never what each fill *netted* after fees.
  Four new fields:
  - `avgRungGapPct` — gap between adjacent same-side rungs.
  - `expectedFeePerRoundtripUsd` — fee for one full rung roundtrip
    (buy + sell, both at maker rate).
  - `breakEvenGapPct` — `2 × feeRateMaker`. Below this gap fees swallow
    gross profit. For HL tier-0 = 3 bps.
  - `feeAwareNetEdgePerRoundtripUsd` — what each completed roundtrip
    actually puts in your pocket.
- **`MarketMeta.feeRateMaker` / `feeRateTaker`** (optional). Defaults
  match HL tier-0 (1.5 / 4.5 bps); override for VIP tiers or fee-free
  venues.
- **Fee-erosion warning** when `avgRungGapPct < 2 × breakEvenGapPct`
  (fees eat ≥ 50% of gross profit per roundtrip).
- 45 self-tests (was 41 in v1.2.3).
- `plan()` semantics, `planHash`, and `levels[]` byte-identical to
  v1.2.3 for the same input. Purely additive output extension.

**Real-world example** from the v1.2.3 live test:
$24 grid, gap 0.275%, maker 1.5 bps:
- gross/RT = $0.034
- fee/RT = $0.0036 (10.6% of gross)
- net/RT = $0.0294
- breakEven = 0.030% (gap is 9× breakEven — comfortable)

## What's new in v1.2.3

- **Notional-aware `quickstart` range** — previously `quickstart`
  recommended the same ±4-7% range regardless of account size, which
  was a disaster for small accounts: 4 forced rungs spread across 4-7%
  meant ~1% gaps between rungs, while hourly vol is ~0.3% — so the
  grid would sit inactive for hours waiting for an outlier move.
  v1.2.3 picks the **tighter** of:
  - **natural geometry** — `(rungs - 1) × σ_hourly × profileGap`, where
    `rungs = clamp(floor(notional × lev / minOrder), 4, 50)`. Each gap
    ≈ one hourly σ, so the grid actually trades on ordinary wiggles.
  - **vol envelope** — the original `k × σ_daily × √7 × profileWidth`,
    kept as an upper bound so big accounts don't blow out to ±15% ranges.
- Real-world impact at σ_d=1.35% (BTC calm day):

  | Account | Pre-1.2.3 | Post-1.2.3 | Source |
  |---|---|---|---|
  | $24 | ±4.4% | **±0.41%** | natural — 4 rungs |
  | $100 | ±4.4% | **±2.61%** | natural — 20 rungs |
  | $5000 | ±4.4% | ±4.36% | vol envelope (unchanged) |

- **`tiny notional` warning** in `quickstart` output when
  `notional × leverage < MIN_GRID_COUNT × minOrder` — the rationale
  shows the exact bump needed to restore full 4-rung concentrated
  coverage instead of falling back to uniform sizing.
- 41 self-tests (was 37 in v1.2.2).
- **`plan()` semantics unchanged.** Only `quickstart()` was modified.
  Users who pass explicit `(rangeLow, rangeHigh)` see no change.

## What's new in v1.2.2

- **Three new plan output fields** for direct UI / agent visibility,
  no need to walk `levels[]`:
  - `buySideNotionalUsd` — sum of `sizeUsd` across all buy rungs.
  - `sellSideNotionalUsd` — sum across all sell rungs. The ratio
    `buySideNotionalUsd / sellSideNotionalUsd` directly reveals the
    funding-bias tilt (positive funding → < 1, negative → > 1).
  - `rangeWidthPct` — `(rangeHigh - rangeLow) / markPrice`. Lets the
    Skill quickly screen "this grid is too wide for the budget" without
    recomputing.
- **`expected fills/day < 1` warning** — when realized vol is too low
  for the configured range, the grid would sit idle and pay funding
  for nothing. The warning suggests a tighter range (≈ ±2σ daily of
  mark) or waiting for higher vol. See `pre-execution check #5` below.
- **REFUSE-prefix abort rule** — explicit pre-execution guard that
  any `plan.warnings` entry starting with `REFUSE:` is a hard stop;
  the agent must not place orders. See `pre-execution check #3` below.
- **planHash unchanged for the same input** — the new output fields
  are derived from existing inputs, so v1.2.2 produces the same
  `planHash` as v1.2.1 for identical input bytes. No migration needed.

37 self-tests (was 33 in v1.2.1).

## What's new in v1.2.1

- **Small-account auto-adapt.** When `totalNotionalUsd × leverage` is too
  small for concentrated-liquidity sizing to keep every rung above
  `marketMeta.minOrderSizeUsd`, the engine now (a) iteratively reduces
  `gridCount` toward `MIN_GRID_COUNT`, then (b) falls back to UNIFORM
  per-rung sizing if even the minimum count can't satisfy the order-size
  floor. Two clear warnings are emitted so the user knows what happened
  and how to restore concentrated geometry (typically: bump notional to
  ≥ $100 or leverage to ≥ 2×). Existing behavior at large notionals is
  unchanged.

### Small-account guidance

Hyperliquid's per-order minimum is typically **$10**. With v1.2's
concentrated-liquidity sizing, edge rungs are ~2-5% of average rung weight,
so a 25-rung grid at $50 notional × 1× leverage produces edge rungs around
$0.05 — below min and rejected on placement.

Real concentration thresholds (assuming 5% wide range, 1.7% daily realized
vol — typical BTC). The Gaussian decay used in concentrated-liquidity
sizing means edge rungs are ~1.6% of center weight, so the "every rung
≥ $10" floor needs surprisingly large notional × leverage:

| Account size × leverage | What user gets | Behavior |
|---|---|---|
| **≥ $1,500 × 1×** OR **≥ $300 × 5×** | Full concentrated, 20-25 rungs | optimal — center rungs 5-10× edge rungs |
| $300-$1,500 × 1× | gridCount auto-reduced to 4 rungs, **then fallback to uniform** | every rung placeable, just no concentration |
| $50-$300 × 1× | Same — 4 rungs uniform (each $12-$75 depending on size) | every order valid, no concentration alpha |
| < $50 × 1× | 4 rungs uniform; may still warn if individual rung < $10 | use ≥ 2× leverage |

**Recommendation for typical $30-$200 accounts:** use `leverage: 2` to 3×
— this multiplies the per-rung effective notional, more easily clearing
$10 minimum *and* enabling concentrated geometry on smaller accounts.
At 2× cross leverage, liquidation buffer is 25% (BTC weekly vol 4-5% —
liquidation is extremely improbable).

**Recommendation for $1,500+ accounts:** stay at 1× for zero-liquidation
safety; the binary will deliver the full 20-25 rung concentrated grid
that captures the most fills per dollar deployed.

## What's new in v1.2.0

- **`hyperliquid-aigrid quickstart`** — zero-friction first-time use. From just
  `coin` + `totalNotionalUsd` + recent `candles` + `marketMeta`, the binary
  derives a sensible `(rangeLow, rangeHigh, leverage, riskProfile)` and
  returns a ready-to-pipe `PlanInput`. Use this when the user describes
  their notional and intent ("$300 BTC grid, conservative") but doesn't
  specify a range. Saves the agent from inventing a range.
- **`hyperliquid-aigrid optimize`** — deterministic parameter sweep. Iterates 75
  combinations (5 range widths × 5 leverages × 3 profiles), runs a
  backtest on each over the recent window, ranks by Calmar score
  (`realizedPnl / max(maxDD, 1)`), returns the top N. Use when the user
  asks "what's the best parameter set" or wants a comparison.

## What's new in v1.1.0

- **Funding-aware sizing.** When the current hourly funding rate is passed
  in `marketMeta.fundingRateHourly`, the grid tilts per-rung notional
  asymmetrically (up to ±20%) to collect funding as alpha. Symmetric when
  funding is missing or below the 10% annualized noise floor.
- **Concentrated-liquidity grid.** Each rung's notional is weighted by its
  estimated one-day fill probability under a log-normal price-move model.
  Rungs near the mark get more capital; rungs at the edges get less. Same
  total notional, higher expected fills per deployed dollar.
- **`hyperliquid-aigrid backtest`.** New binary subcommand that simulates the plan
  candle-by-candle over a historical window and reports realized PnL, max
  drawdown, fill counts, and a Sharpe approximation. Use it before
  `grid-open` to show the user what the current parameters would have
  produced on recent history.

All three upgrades run inside the compiled TypeScript binary — same inputs
still produce byte-identical outputs. `planHash` will differ from v1.0 for
otherwise-identical inputs because the sizing geometry is now concentrated
rather than uniform.

## Commands

### grid-quickstart

Zero-friction first-time use. Given a coin, notional, and recent candles,
derive a sensible plan without the user having to pick a range.

**When to use:** User says something like "$300 BTC grid, conservative"
without specifying a range, or when first-session training wheels are
active. Always prefer `grid-quickstart` over inventing a range yourself.

**Agent execution steps:**

1. Fetch the live `markPrice`, `tickSize`, `minOrderSizeUsd`,
   `maxLeverage` from `hyperliquid-plugin prices` + the static meta.
2. Fetch ~7 days of hourly candles from `https://api.hyperliquid.xyz/info`
   (`type: candleSnapshot`, `interval: 1h`).
3. Pipe into `hyperliquid-aigrid quickstart`:

   ```
   echo '{
     "coin": "BTC",
     "totalNotionalUsd": 300,
     "candles": [...],
     "marketMeta": { "coin": "BTC", "tickSize": 1, "minOrderSizeUsd": 10,
                     "markPrice": 77519, "maxLeverage": 40 },
     "riskProfile": "conservative"
   }' | hyperliquid-aigrid quickstart
   ```

4. The result includes a `planInput` field — pipe that **directly** into
   `hyperliquid-aigrid plan` to get the full GridPlan. Show the user the
   `recommendedRangeLow/High`, `recommendedLeverage`, and the `rationale`
   string before running the plan.

### grid-optimize

Deterministic parameter sweep over (range_width, leverage, risk_profile)
combinations on historical candles.

**When to use:** User asks "what's the optimal parameters" / "find me the
best grid" / "compare profiles", or as a sanity check before running a
hand-picked plan ("how does my chosen params rank in the sweep?").

**Agent execution steps:**

1. Fetch ~30 days of hourly candles from Hyperliquid info (need
   `> backtestWindowBars + 24` so the per-trial backtest has both history
   and a window).
2. Run:

   ```
   echo '{
     "coin": "BTC",
     "totalNotionalUsd": 300,
     "candles": [...],
     "marketMeta": { ... },
     "backtestWindowBars": 168,
     "topN": 3
   }' | hyperliquid-aigrid optimize
   ```

3. Present the top-N candidates as a table: rangeWidth%, leverage,
   profile, fills, realizedPnl, maxDD, score. **Always tell the user**:
   "These are deterministic backtest results on past 7 days, not a
   prediction of future performance. The top candidate is what would have
   maximised Calmar score on recent history."
4. If the user picks one, run `grid-plan` with those exact parameters
   (the candidate already specifies `rangeLow`, `rangeHigh`, `leverage`,
   `riskProfile`).

### grid-plan

Build a `GridPlan` from the user's natural-language description.
**Does not place any orders.**

**When to use:** Whenever the user describes a range-bound view and asks
for a grid, or asks "what would a grid look like for X". Always run before
`grid-open`.

**Inputs parsed from natural language:**

- `instrument` — e.g. `BTC`, `ETH`, `SOL` (perpetual)
- `range_low`, `range_high` — user's expected price band (USD)
- `risk_profile` — `conservative` | `balanced` | `aggressive`
  (default: `conservative`)
- `total_notional_usd` — total capital to commit
- `leverage` — integer, default 2

**Agent execution steps:**

1. Call the Hyperliquid basic plugin to fetch market metadata for
   `instrument`: `tickSize`, `minOrderSizeUsd`, `markPrice`, `maxLeverage`.
   **Also fetch the current funding rate for `instrument` if the basic plugin
   exposes it** (Hyperliquid publishes hourly funding on its `info`
   endpoint — the basic plugin's `prices` or `get-meta` command may return
   it). Pass it through as `marketMeta.fundingRateHourly` (fraction, e.g.
   0.00003 = 3bp/hour ≈ 26% annualized). **v1.1**: when annualized funding
   exceeds 10%, hyperliquid-aigrid tilts per-rung notional asymmetrically (up to ±20%)
   to collect funding as alpha — sell rungs get extra weight under positive
   funding, buy rungs under negative. If the basic plugin does not expose
   funding, omit the field; the engine falls back to a symmetric grid.
2. Call the Hyperliquid basic plugin to fetch recent hourly candles.
   Prefer the last 168 (7 days) for a stable volatility estimate;
   accept as few as 24 (1 day) if that's all the basic plugin returns.
   Fewer than 24 → abort and tell the user volatility can't be
   estimated reliably.
3. **Run `hyperliquid-aigrid plan`**, passing a JSON input with the parsed user
   parameters + market meta + candles. Example:
   ```
   echo '{
     "coin": "BTC",
     "rangeLow": 90000,
     "rangeHigh": 95000,
     "totalNotionalUsd": 300,
     "leverage": 2,
     "riskProfile": "conservative",
     "marketMeta": { ... from basic plugin ... },
     "candles": [ ... from basic plugin ... ]
   }' | hyperliquid-aigrid plan
   ```
4. Parse the returned JSON `GridPlan`. Present it to the user, clearly
   labeled **DRY-RUN PLAN — no orders placed**. Include:
   - Grid count
   - **Both** `plan.totalNotionalUsd` (the full exposure) **and**
     `plan.marginRequiredUsd` (actual USDC margin the user needs =
     notional / leverage). These are different numbers — the agent must
     label them clearly. A $5000 notional plan at 10× only needs $500
     margin; a user who doesn't understand this might fund the wrong
     amount.
   - Leverage
   - Stop-loss trigger price
   - Expected fills/day (from realized volatility)
   - Max loss at range break (absolute and as % of notional)
   - `plan.liquidationDistancePct` — present as **"approx. liquidation
     buffer: X%"** and append the disclaimer: "This is an estimate from
     hyperliquid-aigrid; the authoritative liquidation price will be computed by
     Hyperliquid's risk engine at order time." If the user's grid
     extends close to this estimate, surface that explicitly.
   - Any warnings from `plan.warnings`
   - `plan.planHash` (present as a short identifier, e.g. the first 6
     characters — matching the `ha-{hash[:6]}` form used in the
     strategy tag so users see consistent identifiers across the plan
     view and their order tags). `planHash` is stable across identical
     inputs and can be referenced for support.

**Critical: the agent must never fabricate grid levels or stop-loss
values. Always use the output from `hyperliquid-aigrid plan` verbatim.**

**Tip:** For a human-readable summary of any plan, the agent can run
`hyperliquid-aigrid explain --input <plan.json>` — it returns a plain-English
breakdown with the fields above already formatted. Useful when the
user asks "explain this plan in normal English" or when the raw JSON
is unwieldy.

### grid-backtest

Simulate the plan over historical candles and report what it would have
produced on recent history. **Does not place any orders.**

**When to use:** Before `grid-open`, whenever the user asks "how would
this grid have done over the last week / month" or you want to show them
expected fill counts and PnL-range under the same parameters.

**Agent execution steps:**

1. Fetch a longer hourly-candle window from the Hyperliquid basic plugin
   than you would for planning alone. Recommended: at least 24h of history
   bars (for the volatility estimate) + the backtest window. Example: 1h
   candles for the last 30 days (720 bars total), with
   `backtestWindowBars: 168` to simulate the most recent 7 days.
2. Run `hyperliquid-aigrid backtest`, passing the same inputs as `hyperliquid-aigrid plan` plus
   one extra integer field `backtestWindowBars`:

   ```
   echo '{
     "coin": "BTC",
     "rangeLow": 90000, "rangeHigh": 95000,
     "totalNotionalUsd": 300, "leverage": 2,
     "riskProfile": "balanced",
     "marketMeta": { ... from basic plugin ... },
     "candles": [ ... 720 hourly bars ... ],
     "backtestWindowBars": 168
   }' | hyperliquid-aigrid backtest
   ```

3. Present the `BacktestResult` JSON to the user, clearly labeled as a
   **simulation of past performance on historical data, not a prediction**.
   Include: `fills` (buy/sell split), `realizedPnlUsd`, `unrealizedPnlUsd`,
   `totalPnlUsd`, `maxDrawdownUsd`, `sharpeApprox`, and `hitStopLoss`.
   Always accompany with: "Past performance does not indicate future
   results. Backtest uses a conservative fill model (full-rung fills only,
   no partials); real execution may have different slippage and timing."

4. If the user then says "open it", proceed to `grid-open` with the same
   parameters. `planHash` in the backtest result will match the live plan
   as long as the inputs match.

**Fill model notes:**

- A rung fills when a bar's price range (`[low, high]`) touches the rung's
  price. No partial fills — each rung either fills entirely or waits.
- Buys are processed before sells within the same bar (conservative — biases
  realized PnL lower).
- Stop-loss triggers when a bar's low (for long-bias plans) crosses the
  stop price; simulation ends immediately and realizes the loss on all
  remaining inventory at the trigger price.
- Short-side grids are not simulated in v1.1 — only buy-first, pair-to-sell.
  If the plan is short-biased (`stopLossSide === "short"`), backtest still
  runs but fill counts may under-represent real activity.

### grid-open

Execute the plan produced by `grid-plan` by opening all grid orders
through the Hyperliquid basic plugin.

**When to use:** Only after `grid-plan` has been shown to the user in the
same session and the user has explicitly confirmed ("yes open it", "go
live", or equivalent). Never run without an immediately-prior plan in
context.

**Mandatory pre-execution checks (abort if any fails):**

1. Dry-run flag off (user explicitly went live this session).
2. The `GridPlan` was produced by `hyperliquid-aigrid plan` in this session — never
   hand-constructed.
3. **REFUSE-prefix warnings are a hard stop.** If any element of
   `plan.warnings` starts with the literal string `REFUSE:`, the agent
   MUST abort immediately. Do not call `hyperliquid-plugin order`. Quote
   the warning verbatim to the user and ask them to fix the input
   (typically: refresh the mark price and re-pick a range that contains
   it). The most common trigger is a stale `rangeLow`/`rangeHigh` no
   longer covering the current mark — executing such a plan would open
   a one-sided losing position. There are no overrides; if the user
   insists, they must re-run `hyperliquid-aigrid plan` with a corrected
   range.
4. If `plan.warnings` contains a stop-loss warning (e.g. "stop-loss would
   allow loss of X% > 30% cap"), the agent must:
   - Quote the warning to the user verbatim — do not paraphrase away
     the number.
   - Ask neutrally: "The stop-loss on this plan would allow a larger
     loss than hyperliquid-aigrid's 30% default threshold. Reply **'yes, I'm OK
     with that'** to continue, or **'safer'** to rebuild with lower
     leverage or a tighter range."
   - Only proceed after the user explicitly accepts the risk.
5. If `plan.warnings` contains an `expected fills/day < 1` warning
   (added in v1.2.2), surface it. The grid would sit idle at current
   volatility and pay funding for nothing. Ask the user whether they
   want to (a) tighten the range as suggested in the warning, (b)
   wait for a higher-vol session, or (c) proceed anyway (e.g. if they
   genuinely want a passive limit-ladder rather than an active grid).
6. Present a **final one-line confirmation** and wait for a clear "yes":
   > Opening {gridCount} {coin}-PERP limit orders, total ${totalNotional}
   > at {leverage}x, stop-loss at {stopLossTriggerPrice}. Confirm?

**Execution:**

For each level in `plan.levels`, run:

```bash
hyperliquid-plugin order \
  --coin <plan.coin> \
  --side <level.side> \
  --size <level.sizeCoin> \
  --price <level.price> \
  --order-type limit \
  --strategy-id hyperliquid-aigrid \
  --confirm
```

Notes:

- **`--size` is `level.sizeCoin`** (contract quantity), NOT `level.sizeUsd`.
  `sizeUsd` is for human-readable display only.
- **Always pass `--price`** — without it, market orders use unbounded
  slippage. hyperliquid-aigrid always places resting limit orders at the planned price.
- For per-plan granularity, use `--strategy-id ha-<planHash[:6]>` instead
  (Attribution Rule above documents both forms).
- For high-rung-count plans, prefer `hyperliquid-plugin order-batch`
  (single signed request for many orders) to stay under the rate limit.

After all limit orders succeed, install the stop-loss bracket:

```bash
hyperliquid-plugin tpsl \
  --coin <plan.coin> \
  --sl-px <plan.stopLossTriggerPrice> \
  --strategy-id hyperliquid-aigrid \
  --confirm
```

If any single order is rejected (insufficient margin, price band, rate
limit), stop immediately, quote the exact error from the basic plugin, and
ask the user how to proceed. Do not silently retry. Do not silently skip.

### grid-status

Query the current state of an open grid.

**When to use:** User asks "how is my grid doing", "what's my PnL", "how
many fills so far".

**Identifying the right grid:** If the user has multiple active hyperliquid-aigrid
grids (across different coins or replaced plans), ask them which by
`planHash` (6-char prefix) or by coin. If only one grid is active, no
disambiguation needed.

**Execution:** Call the Hyperliquid basic plugin to fetch open orders
(filter by strategy tag `ha-{planHash[:6]}` if supported), account
state, and fills since the grid was opened. Return: filled count,
realized PnL from filled pairs, unrealized PnL on current position,
distance to stop-loss in absolute price and percentage.

### grid-resume

Reattach to an existing hyperliquid-aigrid grid across sessions.

**When to use:** User comes back after closing their session and asks
"how is my hyperliquid-aigrid grid doing", or wants to manage a grid they don't
remember exactly. Or: a new agent instance needs to take over an
existing grid placed earlier.

**Execution:**

1. Call the Hyperliquid basic plugin to fetch all open orders for the
   user's Hyperliquid account.
2. Filter orders whose strategy tag is `hyperliquid-aigrid` (fixed) or
   starts with `ha-` (per-plan form `ha-{planHash[:6]}`) — these are the
   hyperliquid-aigrid-tagged orders.
3. For per-plan-tagged orders, group by the 6-char `planHash` suffix.
   Each group represents one active grid. Fixed-tag orders may share
   the `hyperliquid-aigrid` tag across multiple grids — disambiguate
   by `coin` + placement timestamp cluster.
4. Present the list to the user: `coin`, `planHash[:6]`, count of open
   orders, current mark price. Let them pick which grid to manage.
5. Once identified, run `grid-status` or `grid-close` as requested.

Note: hyperliquid-aigrid is stateless — the source of truth for any active grid
is the open orders on Hyperliquid, not local storage. The strategy tag
is what makes grids re-discoverable across sessions.

### grid-close

Cancel all open grid orders and optionally flatten the position.

**When to use:** User asks to stop, close, take profit, or cut losses.

**Execution:**

1. Call the Hyperliquid basic plugin to cancel all open orders for the
   instrument. If the basic plugin supports tag-scoped cancellation,
   prefer canceling only orders tagged with `ha-{planHash[:6]}`,
   so other non-hyperliquid-aigrid orders the user might have are preserved.
2. If the user explicitly asks to flatten, call the basic plugin to place
   a market order in the opposite direction of the current position with
   `reduce_only = true` **and strategy tag** `ha-{planHash[:6]}`.
   Warn the user briefly that a market-order flatten on a large
   leveraged position can have meaningful slippage.
3. Confirm completion and report final realized PnL.

### grid-cancel-rung

Cancel one specific rung from an active grid, leaving the others intact.

**When to use:** User asks to cancel a specific rung — e.g. "撤掉最下面那个 buy",
"cancel the $77,608 rung", "drop the bottom buy", "take off the top sell",
"cancel my buy at 77.6k". Use this instead of `grid-close` when the user
wants surgical removal rather than full teardown.

**Disambiguation rules (resolve the user's natural-language reference to one oid):**

The Skill must resolve which rung to cancel from the user's words. Run
`hyperliquid-plugin orders` (filtered to the active hyperliquid-aigrid grid
via `--strategy-id` tag), then map:

| User says | Pick |
|---|---|
| "lowest" / "bottom" / "最下面" | order with smallest `limitPx` |
| "highest" / "top" / "最上面" | order with largest `limitPx` |
| "closest to mid" / "最近的" | order with smallest abs(`limitPx` − markPrice) |
| "furthest" / "最远的" | order with largest abs(`limitPx` − markPrice) |
| "the buy at 77.6k" / explicit price | order whose `limitPx` is closest to the named price (within 1% tolerance) |
| "the sell at $X" | sell-side order whose `limitPx` is closest to X |
| Just "the buy" / "the sell" with one match | the only matching order; if multiple, ASK |

If the description is ambiguous (e.g. "the buy" but multiple buys are
open), enumerate the candidates with their oid + price and let the user
pick — never guess.

**Execution:**

1. Fetch open orders via `hyperliquid-plugin orders --coin <coin>`.
2. Filter to hyperliquid-aigrid-tagged ones (`--strategy-id` match if the
   basic plugin supports the filter; otherwise filter by recent placement
   timestamp + `cloid` heuristics from `grid-resume`).
3. Resolve the user's natural-language reference to exactly one `oid`
   (per the table above). If ambiguous, present candidates and stop.
4. Quote the resolution back to the user verbatim and ask for confirmation:
   > "Cancelling **{side} @ ${price}** (oid {oid}). The other 3 rungs stay
   > active. Confirm?"
5. On confirmation, run:
   ```
   hyperliquid-plugin cancel --coin <coin> --order-id <oid> --confirm
   ```
6. After cancel, optionally fetch the remaining open orders so the user
   sees the updated grid state.

**Side effects to remember:**

- The original `GridPlan`'s `gridCount` no longer matches reality. The
  Skill should NOT auto-rebalance the remaining rungs — that would change
  geometry mid-flight and confuse the audit trail. If the user wants
  to also resize / re-space, recommend `grid-roll` instead.
- Per-rung cancellation does NOT close any already-filled position. If
  the cancelled rung had already filled (creating a position), use
  `grid-close` to flatten or place an explicit reverse market order.
- The cancelled rung is gone — there's no "uncancel". User can place a
  fresh single rung via `hyperliquid-plugin order` if they change their
  mind, but it won't be tied to the original `planHash`.

### grid-roll

Re-center an active grid around the current mark price. Cancels the
existing rungs, recomputes range/levels from current `markPrice` + recent
vol, and places fresh orders. Position (if any) is preserved unless the
user opts to flatten first.

**When to use:** User asks to "shift the grid", "follow the price",
"roll up/down", "tighten the grid", "rebuild around current price",
"按当前价重挂", "网格往上挪". Use this when the user wants the grid to
keep working but with refreshed parameters — typically because:
- Mark drifted to the edge of the original range,
- Vol regime changed (BTC went from 1% σ_d → 3% σ_d → tighter is fine),
- User picked too wide / too tight initially and wants a do-over.

**Execution:**

1. **Pre-flight.** Fetch current `markPrice`, fresh candles, and
   currently-open hyperliquid-aigrid orders. Compute the existing position
   (if any) so the user knows what they're carrying through the roll.

2. **Surface the proposed roll** *before* cancelling anything:
   - Run `grid-quickstart` with current `markPrice` + fresh candles +
     same `totalNotionalUsd` + same `riskProfile` as the existing grid
     (so the new range respects the v1.2.3 notional-aware geometry).
   - Show the user a side-by-side: current rungs vs. proposed new rungs,
     existing position (if any), expected fee per roundtrip, planHash.
   - Ask for confirmation: "Roll the grid to **${newLow}–${newHigh}**?
     Your **{coin} {long/short} {size}** position stays open. Reply
     'roll' to proceed, 'roll + flatten' to also close the position
     first, or 'cancel' to abort."

3. **Cancel old rungs.** On "roll" / "roll + flatten":
   - If "roll + flatten": call `grid-close` first (cancel + reduce-only
     market exit). Confirm flat before continuing.
   - Otherwise: call `hyperliquid-plugin cancel --coin <coin>`
     (or tag-scoped equivalent) to remove all hyperliquid-aigrid rungs.
   - Verify all hyperliquid-aigrid-tagged orders are gone before placing
     new ones. **Never proceed to step 4 if any old rungs remain** —
     that risks double-deployment and accidental fills against stale
     prices.

4. **Place new rungs.** Run `grid-open` on the new `GridPlan` from
   step 2. Each new order carries `--strategy-id hyperliquid-aigrid` and
   the new `planHash[:6]` for attribution.

5. **Report.** Final state: new range, new rung count, new planHash,
   existing position (if not flattened) + its breakeven distance to
   the new range edges.

**Safety rules — abort the roll if any of these:**

- **Mark price out of new range** (per `grid-plan`'s `REFUSE:` warning).
  This shouldn't happen if you used `grid-quickstart` in step 2, but
  guard against it anyway.
- **Existing position would be liquidated by the new range**: compute
  the new stop-loss trigger; if the existing position's entry is on the
  wrong side of it, refuse with: "Your existing position would breach
  the new grid's stop-loss. Either widen the new range, or flatten
  first with 'roll + flatten'."
- **Open orders failed to cancel** (any remaining hyperliquid-aigrid-tagged
  order after step 3): abort. Do NOT place new rungs on top of stale
  ones.
- **Net position size > new grid's `totalNotional × leverage`**: would
  put the account near margin liquidation. Refuse and ask the user to
  reduce position first.

**Why `grid-roll` is one command, not three:**

Without `grid-roll`, users must orchestrate `grid-close` → `grid-plan`
→ `grid-open` themselves, with three confirmation prompts. The compound
flow is error-prone (e.g. cancelling old orders but failing to place
new ones leaves them flat). `grid-roll` makes the transition atomic
from the user's perspective: one decision, one confirmation, one
result. The Skill internally enforces the safety rules above so the
state never gets inconsistent.

## Examples

### Example 1 — Balanced ETH grid, dry-run

**User:** "Plan me a balanced ETH grid between 3200 and 3400, $500 at 2x"

**Agent:**

1. Fetch ETH market meta and candles via Hyperliquid basic plugin.
2. Run `hyperliquid-aigrid plan` with those inputs.
3. Return the plan labeled "DRY-RUN PLAN".
4. End with: "No orders have been placed. Reply 'go live' to execute."

### Example 2 — Aggressive BTC grid with 10× (triggers safety warning)

**User:** "BTC 85k–100k, aggressive, $5000 at 10x, go live"

**Agent:**

1. Run `hyperliquid-aigrid plan`. Output includes warning:
   `stop-loss would allow loss of 52.7% > 30% cap; widen range or tighten
   risk_profile`.
2. Present the full plan. Quote the warning verbatim — do not paraphrase
   away the number.
3. Ask neutrally: "At 10× leverage across this range, a stop-out would
   cost about 53% of your margin. hyperliquid-aigrid's default threshold for an
   automatic proceed is 30%. Reply **'yes, I'm OK with that'** to
   continue as planned, or **'safer'** to get the same plan with lower
   leverage."
4. Only if the user explicitly accepts the risk: present the final
   one-line confirmation and then execute via the basic plugin.
5. If user replies "safer": re-run `hyperliquid-aigrid plan` with `leverage: 5` (or
   one step down from the user's original) and loop back to step 1.

### Example 3 — User asks for something beyond all caps

**User:** "Open a 50x BTC grid with $50,000"

**Agent:**

1. Refuse silently clamping. Explain: leverage capped at 10x by hyperliquid-aigrid,
   notional capped at $5000 by hyperliquid-aigrid.
2. Offer: "I can open up to a 10x $5000 grid. That's the hard safety
   ceiling of hyperliquid-aigrid. Want to proceed at 10x $5000?"
3. Never compute or execute a plan above the caps — the binary would
   clamp them silently and produce a warning, but the agent must stop
   before even running `hyperliquid-aigrid plan` in this case.

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Hyperliquid basic plugin not installed" | Basic plugin missing | Tell user to run `npx skills add okx/plugin-store --skill hyperliquid-plugin` |
| "hyperliquid-aigrid: command not found" | Binary not on PATH | Re-install this Skill via `npx skills add okx/plugin-store --skill hyperliquid-aigrid`; verify `hyperliquid-aigrid --version` |
| `hyperliquid-aigrid error: ...` | Invalid input JSON, missing fields | Show the binary's error verbatim; ask the user to rephrase or correct the input |
| Plan has `warnings` about notional clamp | User asked for > $5000 | Surface the warning; confirm user wants to proceed at the clamped cap |
| Plan has `warnings` about leverage clamp | User asked for > 10× | Surface the warning; confirm user wants to proceed at 10× |
| Plan has `warnings` about "auto-downshifted" to 5× | User requested conservative + leverage > 5× | Explain: conservative profile caps leverage at 5× for safety; if they want higher leverage, suggest switching to balanced or aggressive and re-plan |
| Plan has `warnings` about stop-loss > 30% | High leverage + loose range | See Example 2 above — require explicit "yes, I'm OK with that" before executing |
| Plan has `warnings` starting with **REFUSE** | User's range premise contradicts the live mark price | Hard refuse — do **not** execute. Tell the user their range appears stale or wrong and ask them to recheck before re-planning |
| Plan has `warnings` about mark/candle drift | Mark price drifted >3% from last candle close | Surface the warning. Either refetch fresh candles and re-plan, or ask the user to confirm they understand the volatility estimate may be stale |
| Plan has `warnings` about candle gaps | Hyperliquid basic plugin returned a non-contiguous candle series | Refetch candles. If the gap persists, note it in the user-facing plan and add caution about the fills/day estimate |
| Plan has `warnings` about per-level size below market min | Notional too small for grid_count + price | Reduce grid_count (e.g. switch from aggressive to balanced) OR increase notional. Rerun `hyperliquid-aigrid plan` |
| Plan has empty `levels` array | Invalid range, price band too tight | Quote the warning; ask user to widen the range |
| `planHash` differs between two runs | Candles or marketMeta changed between fetches | Expected behavior — `planHash` hashes over the full plan output including levels derived from live data. If the user wants the *same* plan as before, use the same market snapshot (cached); otherwise explain that prices moved, so the plan is legitimately different. |
| "Insufficient margin" from basic plugin | Account balance too low | Show `plan.marginRequiredUsd` (not notional) as the amount the user needs to deposit; do not silently reduce order sizes |
| "Order rejected by Hyperliquid" | Price band / post-only / rate limit | Quote the basic plugin's exact error; stop; ask user how to proceed |
| "Rate limited" from basic plugin | Too many orders too quickly | Wait 2 seconds; continue with remaining levels; never drop orders to stay under the rate limit |
| "Price moved during placement" | Mark drifted mid-execution | Pause; report the drift; ask the user whether to continue, adjust, or cancel |

## Security Notices

**Risk level: advanced.** This Skill places and manages live perpetual
orders on Hyperliquid. Perpetual futures are leveraged instruments. Losses
can equal or, in adverse conditions, exceed the collateral committed.

**Hard safety limits built into the `hyperliquid-aigrid` binary:**

- Maximum **$5,000** total notional per grid.
- Maximum **10×** leverage.
- Maximum **50** grid rungs.
- Stop-loss sized to bound worst-case loss at **≤ 30%** of notional (when
  this cap would be breached, the binary emits a warning and this Skill
  requires explicit user acknowledgment before executing).

These limits are enforced in the binary's source code
(`src/types.ts:CAPS`). They cannot be overridden from within this Skill.
A user who wants higher limits must use a different Skill.

**Behavioral safety:**

- **Dry-run mode by default.** Every new session starts in dry-run.
- **Explicit final confirmation** required before any order batch.
- **Deterministic planning.** The grid math is done in a compiled binary,
  not by the LLM — so the same inputs always produce the same plan. The
  agent cannot silently fabricate or alter grid levels.

**Circuit breakers (enforced by this Skill's pre-flight — see Pre-flight
Checks §6):**

- **Same-instrument lockout.** If the user already has an active hyperliquid-aigrid
  grid on the same coin, refuse to open a second one unless the user
  explicitly replaces it.
- **Daily loss stop.** If cumulative realized PnL from hyperliquid-aigrid-tagged
  trades over the last 24 hours is worse than −2% of the user's account
  equity, refuse to open a new grid until the next UTC day.
- **Consecutive loss stop.** If the user's last two hyperliquid-aigrid-tagged grid
  closures both resulted in net realized loss, refuse to open a new grid
  and recommend they pause hyperliquid-aigrid for the day and reassess the market
  before continuing.

These are risk guardrails, not regulatory limits — the Skill should
explain clearly *why* it is refusing and what would need to change to
retry, not just block silently.

**What this Skill does not do:**

- It does not hold or touch private keys. All signing happens inside the
  Agentic Wallet's Trusted Execution Environment, via the Hyperliquid
  basic plugin.
- It does not collect, store, or transmit user wallet addresses, balances,
  or trade history to any external server.
- It makes no external API calls outside the Hyperliquid basic plugin
  tool surface (`api_calls: []` in `plugin.yaml`).
- The binary itself makes no network calls — it is a pure compute step.

**Disclaimer:** Grid strategies assume mean-reverting, range-bound price
action. In a trending or breakout market, a grid accumulates a losing
position against the trend; the stop-loss mitigates but does not
eliminate this risk. Leverage amplifies both gains and losses. Nothing in
this Skill is financial, investment, or trading advice. You are
responsible for your own sizing, risk management, and decision to trade.

## Skill Routing

- **For on-chain Hyperliquid operations only (single orders, no strategy
  wrapper)** → use the Hyperliquid basic plugin (`hyperliquid-plugin`)
  directly.
- **For portfolio / balance overview across chains** → use OKX's portfolio
  skills from Onchain OS.
- **For DEX swaps on other chains** → use OKX DEX skills from Onchain OS.
