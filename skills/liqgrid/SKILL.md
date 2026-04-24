---
name: liqgrid
description: "Natural-language perpetual grids on Hyperliquid with funding-aware asymmetric sizing, concentrated-liquidity weighting, and a deterministic backtest engine — all on top of hyperliquid-plugin."
version: "1.1.0"
author: "dddd86971-cloud"
tags:
  - hyperliquid
  - grid
  - perpetuals
  - strategy
  - agentic-wallet
---

# liqgrid

## Overview

`liqgrid` turns a trader's one-sentence view on a Hyperliquid perpetual —
"BTC will chop between 90k and 95k, aggressive grid, $3000 at 5x" — into a
fully specified, risk-capped grid strategy.

What makes liqgrid different from plain Skill-based grids:

- A **deterministic TypeScript engine** (the `liqgrid` binary, shipped with
  this Skill) computes the grid plan. Same inputs → same output, every
  time, across runs and models.
- The LLM agent handles **natural-language parsing** and **live market
  data fetching** (through the Hyperliquid basic plugin). The engine
  handles **math**. Clear separation of responsibilities.
- All on-chain writes go through the **Hyperliquid basic plugin**, inside
  the Agentic Wallet. liqgrid never bypasses it.

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

- the user is new to liqgrid (first session, no prior grids)
- the user expresses caution, mentions "small", "safe", or doesn't
  specify a notional larger than $500
- the user chose leverage ≥ 5× (compound aggressive-leverage and
  aggressive-density = unnecessary risk)
- the market's recent realized volatility is unusually high (>5% daily)

Use `balanced` when:

- the user has run liqgrid before without issue
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
liqgrid-tagged fills in account history), prefer this introduction:

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
3. The `liqgrid` binary is available on PATH (installed automatically when
   this Skill is installed via `npx skills add okx/plugin-store --skill
   liqgrid`). Verify with `liqgrid --version`.
4. The user has funded their Hyperliquid perp account with at least the
   minimum collateral for the chosen instrument (typically ≥ 20 USDC).
5. **No existing active liqgrid grid on the same instrument.** Call the
   Hyperliquid basic plugin to check open orders; if any orders tagged
   as liqgrid (see Attribution Rule below) are open on the requested
   instrument, refuse to open a second grid there unless the user
   explicitly says "replace the existing grid". Stacking multiple
   liqgrid grids on the same instrument can produce offsetting orders
   and unpredictable P&L.
6. **Circuit-breaker state is clear.** Call the Hyperliquid basic plugin
   to fetch the user's fills from the last 24 hours. If cumulative
   realized PnL from liqgrid-tagged trades is worse than −2% of
   account equity, refuse to open a new grid until the next UTC day.
   Tell the user why and when they can retry.
7. **Dry-run mode is the default.** Every new session starts in dry-run.
   Live execution requires an explicit user confirmation in natural
   language ("go live", "place real orders", etc.).

## Attribution Rule

Every write operation routed to the Hyperliquid basic plugin must include
the strategy tag `liqgrid` in whatever tag/cloid/strategy-id field the
basic plugin exposes:

- If the basic plugin accepts a `strategy_id`, `tag`, or `clientOrderId`
  parameter, pass `liqgrid` (or `lg-{planHash[:6]}` for per-plan
  granularity).
- If the basic plugin exposes a `--strategy-id` CLI flag, append
  `--strategy-id liqgrid`.
- If no such field exists, embed `liqgrid` in any free-form order-label
  field available.

Read-only market inspection (getting mark price, candles, meta) does not
need tagging.

This tagging is what lets the Plugin Store Season 1 Challenge leaderboard
correctly attribute trades to this Skill. Untagged trades may be
aggregated to the generic `hyperliquid-plugin` and not counted toward
liqgrid's leaderboard position.

## Tool Name Adaptation

This Skill refers to Hyperliquid basic-plugin operations by their semantic
purpose (`get_market_meta`, `get_candles`, `place_order`,
`place_trigger_order`, `cancel_all_orders`, `get_open_orders`,
`get_user_state`, `get_user_fills`). If the basic plugin exposes these
under different names at runtime, the agent should map to the real names
using the basic plugin's own documentation while preserving the semantics
described here. Under no circumstances should the agent call Hyperliquid
directly via HTTP, RPC, or any other non-plugin path — all on-chain writes
must go through the Hyperliquid basic plugin.

## What's new in v1.1.0

- **Funding-aware sizing.** When the current hourly funding rate is passed
  in `marketMeta.fundingRateHourly`, the grid tilts per-rung notional
  asymmetrically (up to ±20%) to collect funding as alpha. Symmetric when
  funding is missing or below the 10% annualized noise floor.
- **Concentrated-liquidity grid.** Each rung's notional is weighted by its
  estimated one-day fill probability under a log-normal price-move model.
  Rungs near the mark get more capital; rungs at the edges get less. Same
  total notional, higher expected fills per deployed dollar.
- **`liqgrid backtest`.** New binary subcommand that simulates the plan
  candle-by-candle over a historical window and reports realized PnL, max
  drawdown, fill counts, and a Sharpe approximation. Use it before
  `grid-open` to show the user what the current parameters would have
  produced on recent history.

All three upgrades run inside the compiled TypeScript binary — same inputs
still produce byte-identical outputs. `planHash` will differ from v1.0 for
otherwise-identical inputs because the sizing geometry is now concentrated
rather than uniform.

## Commands

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
   exceeds 10%, liqgrid tilts per-rung notional asymmetrically (up to ±20%)
   to collect funding as alpha — sell rungs get extra weight under positive
   funding, buy rungs under negative. If the basic plugin does not expose
   funding, omit the field; the engine falls back to a symmetric grid.
2. Call the Hyperliquid basic plugin to fetch recent hourly candles.
   Prefer the last 168 (7 days) for a stable volatility estimate;
   accept as few as 24 (1 day) if that's all the basic plugin returns.
   Fewer than 24 → abort and tell the user volatility can't be
   estimated reliably.
3. **Run `liqgrid plan`**, passing a JSON input with the parsed user
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
   }' | liqgrid plan
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
     liqgrid; the authoritative liquidation price will be computed by
     Hyperliquid's risk engine at order time." If the user's grid
     extends close to this estimate, surface that explicitly.
   - Any warnings from `plan.warnings`
   - `plan.planHash` (present as a short identifier, e.g. the first 6
     characters — matching the `lg-{hash[:6]}` form used in the
     strategy tag so users see consistent identifiers across the plan
     view and their order tags). `planHash` is stable across identical
     inputs and can be referenced for support.

**Critical: the agent must never fabricate grid levels or stop-loss
values. Always use the output from `liqgrid plan` verbatim.**

**Tip:** For a human-readable summary of any plan, the agent can run
`liqgrid explain --input <plan.json>` — it returns a plain-English
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
2. Run `liqgrid backtest`, passing the same inputs as `liqgrid plan` plus
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
   }' | liqgrid backtest
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
2. The `GridPlan` was produced by `liqgrid plan` in this session — never
   hand-constructed.
3. If `plan.warnings` contains a stop-loss warning (e.g. "stop-loss would
   allow loss of X% > 30% cap"), the agent must:
   - Quote the warning to the user verbatim — do not paraphrase away
     the number.
   - Ask neutrally: "The stop-loss on this plan would allow a larger
     loss than liqgrid's 30% default threshold. Reply **'yes, I'm OK
     with that'** to continue, or **'safer'** to rebuild with lower
     leverage or a tighter range."
   - Only proceed after the user explicitly accepts the risk.
4. Present a **final one-line confirmation** and wait for a clear "yes":
   > Opening {gridCount} {coin}-PERP limit orders, total ${totalNotional}
   > at {leverage}x, stop-loss at {stopLossTriggerPrice}. Confirm?

**Execution:**

For each level in `plan.levels`, call the Hyperliquid basic plugin's
order-placing tool with:
- `coin = plan.coin`
- `side = level.side` (`"buy"` or `"sell"`)
- `price = level.price`
- `size = level.sizeCoin` — Hyperliquid's order API takes contract
  quantity, not USD notional. Use `sizeCoin`. `sizeUsd` in the plan is
  only for human-readable display and must not be passed as the order
  size.
- `order_type = "limit"`
- `post_only = true`
- `reduce_only = false`
- **strategy tag**: `lg-{planHash[:6]}` (see Attribution Rule).
  This is what makes the orders attributable to this Skill on the
  Plugin Store leaderboard.

After all limit orders succeed, call the Hyperliquid basic plugin's
trigger-order tool to install a stop-market at
`plan.stopLossTriggerPrice` with `reduce_only = true` **and the same
strategy tag** `lg-{planHash[:6]}`.

If any single order is rejected (insufficient margin, price band, rate
limit), stop immediately, quote the exact error from the basic plugin, and
ask the user how to proceed. Do not silently retry. Do not silently skip.

### grid-status

Query the current state of an open grid.

**When to use:** User asks "how is my grid doing", "what's my PnL", "how
many fills so far".

**Identifying the right grid:** If the user has multiple active liqgrid
grids (across different coins or replaced plans), ask them which by
`planHash` (6-char prefix) or by coin. If only one grid is active, no
disambiguation needed.

**Execution:** Call the Hyperliquid basic plugin to fetch open orders
(filter by strategy tag `lg-{planHash[:6]}` if supported), account
state, and fills since the grid was opened. Return: filled count,
realized PnL from filled pairs, unrealized PnL on current position,
distance to stop-loss in absolute price and percentage.

### grid-resume

Reattach to an existing liqgrid grid across sessions.

**When to use:** User comes back after closing their session and asks
"how is my liqgrid grid doing", or wants to manage a grid they don't
remember exactly. Or: a new agent instance needs to take over an
existing grid placed earlier.

**Execution:**

1. Call the Hyperliquid basic plugin to fetch all open orders for the
   user's Hyperliquid account.
2. Filter orders whose strategy tag starts with `lg-` — these are
   liqgrid-tagged orders.
3. Group by the tag suffix (the 6-char planHash prefix). Each group
   represents one active grid.
4. Present the list to the user: `coin`, `planHash[:6]`, count of open
   orders, current mark price. Let them pick which grid to manage.
5. Once identified, run `grid-status` or `grid-close` as requested.

Note: liqgrid is stateless — the source of truth for any active grid
is the open orders on Hyperliquid, not local storage. The strategy tag
is what makes grids re-discoverable across sessions.

### grid-close

Cancel all open grid orders and optionally flatten the position.

**When to use:** User asks to stop, close, take profit, or cut losses.

**Execution:**

1. Call the Hyperliquid basic plugin to cancel all open orders for the
   instrument. If the basic plugin supports tag-scoped cancellation,
   prefer canceling only orders tagged with `lg-{planHash[:6]}`,
   so other non-liqgrid orders the user might have are preserved.
2. If the user explicitly asks to flatten, call the basic plugin to place
   a market order in the opposite direction of the current position with
   `reduce_only = true` **and strategy tag** `lg-{planHash[:6]}`.
   Warn the user briefly that a market-order flatten on a large
   leveraged position can have meaningful slippage.
3. Confirm completion and report final realized PnL.

## Examples

### Example 1 — Balanced ETH grid, dry-run

**User:** "Plan me a balanced ETH grid between 3200 and 3400, $500 at 2x"

**Agent:**

1. Fetch ETH market meta and candles via Hyperliquid basic plugin.
2. Run `liqgrid plan` with those inputs.
3. Return the plan labeled "DRY-RUN PLAN".
4. End with: "No orders have been placed. Reply 'go live' to execute."

### Example 2 — Aggressive BTC grid with 10× (triggers safety warning)

**User:** "BTC 85k–100k, aggressive, $5000 at 10x, go live"

**Agent:**

1. Run `liqgrid plan`. Output includes warning:
   `stop-loss would allow loss of 52.7% > 30% cap; widen range or tighten
   risk_profile`.
2. Present the full plan. Quote the warning verbatim — do not paraphrase
   away the number.
3. Ask neutrally: "At 10× leverage across this range, a stop-out would
   cost about 53% of your margin. liqgrid's default threshold for an
   automatic proceed is 30%. Reply **'yes, I'm OK with that'** to
   continue as planned, or **'safer'** to get the same plan with lower
   leverage."
4. Only if the user explicitly accepts the risk: present the final
   one-line confirmation and then execute via the basic plugin.
5. If user replies "safer": re-run `liqgrid plan` with `leverage: 5` (or
   one step down from the user's original) and loop back to step 1.

### Example 3 — User asks for something beyond all caps

**User:** "Open a 50x BTC grid with $50,000"

**Agent:**

1. Refuse silently clamping. Explain: leverage capped at 10x by liqgrid,
   notional capped at $5000 by liqgrid.
2. Offer: "I can open up to a 10x $5000 grid. That's the hard safety
   ceiling of liqgrid v1.0.0. Want to proceed at 10x $5000?"
3. Never compute or execute a plan above the caps — the binary would
   clamp them silently and produce a warning, but the agent must stop
   before even running `liqgrid plan` in this case.

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Hyperliquid basic plugin not installed" | Basic plugin missing | Tell user to run `npx skills add okx/plugin-store --skill hyperliquid-plugin` |
| "liqgrid: command not found" | Binary not on PATH | Re-install this Skill via `npx skills add okx/plugin-store --skill liqgrid`; verify `liqgrid --version` |
| `liqgrid error: ...` | Invalid input JSON, missing fields | Show the binary's error verbatim; ask the user to rephrase or correct the input |
| Plan has `warnings` about notional clamp | User asked for > $5000 | Surface the warning; confirm user wants to proceed at the clamped cap |
| Plan has `warnings` about leverage clamp | User asked for > 10× | Surface the warning; confirm user wants to proceed at 10× |
| Plan has `warnings` about "auto-downshifted" to 5× | User requested conservative + leverage > 5× | Explain: conservative profile caps leverage at 5× for safety; if they want higher leverage, suggest switching to balanced or aggressive and re-plan |
| Plan has `warnings` about stop-loss > 30% | High leverage + loose range | See Example 2 above — require explicit "yes, I'm OK with that" before executing |
| Plan has `warnings` starting with **REFUSE** | User's range premise contradicts the live mark price | Hard refuse — do **not** execute. Tell the user their range appears stale or wrong and ask them to recheck before re-planning |
| Plan has `warnings` about mark/candle drift | Mark price drifted >3% from last candle close | Surface the warning. Either refetch fresh candles and re-plan, or ask the user to confirm they understand the volatility estimate may be stale |
| Plan has `warnings` about candle gaps | Hyperliquid basic plugin returned a non-contiguous candle series | Refetch candles. If the gap persists, note it in the user-facing plan and add caution about the fills/day estimate |
| Plan has `warnings` about per-level size below market min | Notional too small for grid_count + price | Reduce grid_count (e.g. switch from aggressive to balanced) OR increase notional. Rerun `liqgrid plan` |
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

**Hard safety limits built into the `liqgrid` binary:**

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

- **Same-instrument lockout.** If the user already has an active liqgrid
  grid on the same coin, refuse to open a second one unless the user
  explicitly replaces it.
- **Daily loss stop.** If cumulative realized PnL from liqgrid-tagged
  trades over the last 24 hours is worse than −2% of the user's account
  equity, refuse to open a new grid until the next UTC day.
- **Consecutive loss stop.** If the user's last two liqgrid-tagged grid
  closures both resulted in net realized loss, refuse to open a new grid
  and recommend they pause liqgrid for the day and reassess the market
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
