---
name: livermore-perps
description: "Time-distributed exit for Hyperliquid perpetual positions. Splits a position close into N slices over a configurable duration to minimize market impact and slippage on thin liquidity pairs. Use when user says: 'TWAP exit my BTC position', 'scale out of my hyperliquid short', 'close my SOL perp slowly over 30 minutes', 'don't dump my position', 'walk me out of my HL position', 'gradual exit', 'iceberg close', 'avoid slippage on my hyperliquid close', 'exit strategy that doesn't move the market'. Do NOT use for: opening new positions (use hyperliquid-plugin); spot trading (use hyperliquid-plugin spot or okx-dex-swap); standard market close (use hyperliquid-plugin close); rule-based exit triggers like trailing stops or kill-switch (those are entry-condition-driven, not size-driven). Exit-only by design — never opens positions, never increases leverage."
version: "1.0.0"
author: "Maxim Voshchansky"
tags:
  - hyperliquid
  - perpetuals
  - twap
  - scaled-exit
  - market-impact
  - slippage-control
  - livermore
---

# Livermore Perps — Time-Distributed Exit for Hyperliquid

> *"It was never my thinking that made the big money for me. It always was my sitting."* — Jesse Livermore
>
> Livermore traded blocks of stock too large for the market to swallow in one bite. He learned to feed positions to the orderbook in pieces, over hours, until the market never noticed he was leaving. This skill is that lesson, applied to perpetual futures.

A **scaled-exit primitive** for Hyperliquid perpetuals. It does not open positions. It does not pick exit triggers. When you decide it is time to leave, it splits your close into N reduce-only orders over a configurable duration — minimizing market impact and slippage on thin liquidity pairs.

This is the perpetuals companion to **OnchainLivermore** (spot exit-discipline on Solana and X Layer). Same philosophy, different venue. You decide when to enter and when to leave. Livermore decides **how** to leave — slice by slice.

## Disclaimer

**This skill is for educational and research purposes only and does not constitute investment, trading, or financial advice.**

1. **Perpetual futures involve extreme risk.** Leverage amplifies both gains and losses. Liquidation can result in **total loss of margin**.
2. **TWAP does not guarantee a better fill than a single market order.** It guarantees that fills are distributed across time. In a directionally moving market, average fill price can be worse than a single market exit.
3. **Slippage and partial fills are still possible** on each individual slice. The skill caps maximum slippage per slice but cannot prevent execution failure during extreme volatility.
4. **Dry-run mode is the default.** Live execution requires explicit user confirmation per session.

By using this skill, you accept that all on-chain transactions and trading outcomes are your sole responsibility, consistent with [OKX Web3 Ecosystem Terms of Service §4.10](https://web3.okx.com/help/okx-web3-build-user-agreement) and the Hyperliquid trading agreement.

## Overview

When a perp trader holds a position that is large relative to the order book — a $50k BTC long on a midcap derivatives pair, a $200k position on a low-liquidity altcoin perp — closing it with a single market order can move the price 0.5–2% against the trader. That is real money, paid as slippage, on every single exit.

The professional answer is **TWAP** (Time-Weighted Average Price): split the close into N smaller orders distributed over a duration. Each slice is small enough that the orderbook absorbs it without flinching. The composite fill price is closer to the weighted-average market price during the window.

CEX traders have had this primitive for decades. Hyperliquid does not provide it natively. This skill adds it.

## Pre-flight Checks

Before using this skill, ensure:

1. **`onchainos` CLI is installed** — `npx skills add okx/onchainos-skills`
2. **`hyperliquid-plugin` is installed** — `npx skills add okx/plugin-store --skill hyperliquid-plugin`
3. **Active Agentic Wallet session** — sign in via `onchainos wallet login`
4. **At least one open Hyperliquid perp position** — this is an *exit* skill; without positions, there is nothing to walk out.

If `hyperliquid-plugin` is not installed, this skill cannot operate. Surface that clearly and stop — there is no fallback.

## Trigger Phrases

The skill activates on phrases like:
- "TWAP exit my BTC position"
- "scale out of my hyperliquid short over 30 minutes"
- "don't dump my position into the book"
- "walk me out of my HL position slowly"
- "iceberg close on ETH perp"
- "exit my position without moving the market"
- 中文："分批退出我的 hyperliquid 仓位"，"分时段平仓"

## When to Use TWAP vs Single Market Close

This skill is not always the right answer. A heuristic for the agent:

| Position size relative to book | Recommended exit |
|---|---|
| Less than 0.1% of 1% book depth | `hyperliquid close` (single market order is fine) |
| 0.1% to 1% of 1% book depth | `livermore-perps start` with 5–10 slices over 5–15 min |
| 1% to 5% of 1% book depth | `livermore-perps start` with 15–30 slices over 30–60 min |
| Over 5% of 1% book depth | `livermore-perps start` with 30+ slices over 1–4 hours; consider manual review |

When in doubt, run `simulate` first to estimate market impact before committing.

If the user has set a hard deadline ("close in next 5 minutes regardless of impact"), TWAP may not be the right tool — recommend `hyperliquid close` directly.

## Commands

### 1. `start` — Begin a TWAP exit

```bash
onchainos livermore-perps start --coin <COIN> --duration <DURATION> --slices <N> [--side close-long|close-short] [--max-slippage-per-slice <PCT>] [--dry-run true|false] [--confirm]
```

**When to use**: User wants to gradually close an open perp position over time.

**Required parameters**:
- `--coin BTC` — coin symbol of the position to close
- `--duration 30m` — total time over which to distribute slices (`5m`, `30m`, `1h`, `2h`, `4h` accepted; minimum 5m, maximum 4h without `--force`)
- `--slices 15` — number of distinct orders to place (minimum 3, maximum 60)

**Optional parameters**:
- `--side close-long` or `--side close-short` — auto-detected from current position if omitted
- `--max-slippage-per-slice 0.5` — abort a slice if the quoted price differs from mid by more than this percent (default 1.0)
- `--dry-run true` (default) — simulate slice schedule without placing orders
- `--confirm` — required for live execution (i.e., when `--dry-run false`)

**Pre-execution checks**:
1. Verify the position exists and matches the requested side via `hyperliquid positions`
2. Compute slice size: `position_size / slices`, rounded to coin's lot size
3. Compute schedule: first slice at `t=0`, then evenly spaced over `duration`
4. Estimate market impact using `hyperliquid prices` and standard book-depth assumptions
5. If the estimated impact is **lower** with a single market order than with TWAP (e.g., position is small or duration is too short to matter), warn the user and offer to fall back to `hyperliquid close` instead

**During execution**:
1. For each scheduled slice, call `hyperliquid order --coin <C> --side <S> --size <slice_size> --type market --reduce_only true --slippage <max-slippage-per-slice> --confirm`
2. After each slice fills (or fails), append a journal entry with: timestamp, slice number, slice size, fill price, slippage realized, cumulative fill, remaining size
3. If a slice fails due to slippage abort: log the failure, **do not retry on the same slice clock**, continue the schedule for remaining slices. The skipped size is added to the final slice (catch-up) or — if `--strict-schedule` is set — abandoned and surfaced to user.

**Output**: A live progress table updated after each slice — `[3/15] BTC sold 0.0033 @ $103,250 (slippage 0.08%, cumulative 0.0099 BTC)` and final summary on completion.

### 2. `status` — Check active TWAP exits

```bash
onchainos livermore-perps status [--coin <COIN>]
```

**When to use**: User wants to see progress on running TWAP exits without interrupting them.

**Output**: Per-active-TWAP table — coin, side, slices completed/total, time elapsed/total, cumulative size filled, average fill price, current VWAP estimate, next slice ETA.

### 3. `cancel` — Stop a running TWAP

```bash
onchainos livermore-perps cancel --coin <COIN> [--reason <TEXT>]
```

**When to use**: User changes mind mid-exit (market reversed favorably, news event, manual takeover).

**Behavior**: Stops the slice scheduler. Already-placed orders that have filled cannot be reversed. The remaining size of the position is **not** automatically closed — it stays open. User can manually `hyperliquid close` the rest, restart a new TWAP, or hold.

The `--reason` flag is optional but recorded in the journal for later review. Highly recommended: writing one sentence about why now teaches you something later.

### 4. `simulate` — Dry-run a TWAP plan

```bash
onchainos livermore-perps simulate --coin <COIN> --duration <DURATION> --slices <N>
```

**When to use**: User wants to see what would happen without committing.

**Output**:
- Slice-by-slice schedule (timestamps, sizes)
- Estimated total slippage based on current book depth
- Comparison: estimated cost of single-market-close vs estimated cost of this TWAP plan
- Recommendation: "TWAP saves an estimated $X" or "Single market close is cheaper for this size — consider `hyperliquid close` instead"

### 5. `log` — Decision and execution journal

```bash
onchainos livermore-perps log [--coin <COIN>] [--last <N>]
```

**When to use**: User wants to see history of past TWAP exits and how they performed.

**Output**: Reverse-chronological list. Each TWAP run is one block: started-at timestamp, parameters used, slices completed, average fill, total realized PnL, slippage observed, comparison to single-market-close estimate.

## Slice Sizing and Schedule

Slices are equal-sized by default: `size_per_slice = floor(total_size / slices, lot_size)`. The remainder (if any) is added to the **final** slice, not the first — exposure is reduced earliest, and any rounding tail is left to last when the position is already mostly closed.

Slice timing is uniform: first slice at `t=0`, last slice at `t = duration * (slices-1) / slices`. The skill does **not** randomize slice timing — predictable schedules are auditable, and the marginal anti-detection benefit of randomization on a perp DEX is low.

Adaptive sizing (smaller slices when book is thin, larger when book is deep) is **not** in v1.0. It was considered and rejected for this release because it adds complexity without an obvious win on Hyperliquid's relatively uniform book structure. Future versions may revisit.

## Decision Journal Format

Every slice writes one journal entry with the same shape:

```
[2026-05-08 14:32:11 UTC] BTC TWAP exit — slice 7/15
  Plan: 30 min, 15 slices, 0.0033 BTC each
  Slice: market sell 0.0033 BTC, max-slippage 0.5%, reduce_only=true
  Fill: 0.0033 BTC @ $103,180 (mid was $103,200, slippage 0.019%)
  Cumulative: 0.0231 / 0.0495 BTC closed (47% complete)
  Next slice: 14:34:11 UTC
  Reason: Scheduled execution. Slippage well within tolerance.
```

Stored at `~/.livermore-perps/journal.jsonl`, append-only, one entry per line.

## Fail-safe Principle

**Ambiguous failures resolve toward continuing the schedule, not aborting.** A TWAP exit that completes 12 of 15 slices is better than one that aborts at slice 4 because of one bad fill.

Specific behaviors:

- **Single slice fails slippage check** → log, skip, continue. Skipped size accumulates and is added to the **final** slice.
- **Multiple consecutive slices fail (3+ in a row)** → pause the schedule, surface to user with options: continue at relaxed slippage, abort and manually close, or fall back to a single market close of remaining size.
- **`hyperliquid prices` returns stale** → use last known price for slippage check; if data is more than 60 seconds stale, pause the schedule and surface to user.
- **`hyperliquid order` returns network error** → retry the slice once with 5-second backoff. On second failure, log and skip (move to next slice).
- **Position size on Hyperliquid drops below expected** mid-TWAP (user manually closed part externally) → recompute remaining size from `hyperliquid positions` before each slice. The TWAP adapts; it does not assume the world stood still.
- **Position fully closes mid-TWAP** (e.g., user hit `hyperliquid close` separately, or position was liquidated) → cancel remaining schedule, write a clear journal entry. No further slices are attempted on a non-existent position.
- **The user invokes `cancel`** → stop the scheduler immediately. Already-filled slices stand; remaining size is left open. This is a user choice, not a fail-state.

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "hyperliquid-plugin not installed" | Required dependency missing | Run `npx skills add okx/plugin-store --skill hyperliquid-plugin` |
| "No open position found for COIN" | Nothing to TWAP-exit | Open a position via `hyperliquid order` first |
| "Position side does not match --side" | User specified wrong direction | Re-run with auto-detected side (omit `--side`) |
| "Slice size below minimum lot" | `slices` too high for position size | Reduce `--slices` or increase position size |
| "Duration below minimum (5m)" | TWAP too short to matter | Use `hyperliquid close` for instant exit, or extend duration |
| "Duration above maximum (4h) without --force" | TWAP unreasonably long | Add `--force` to override (use cautiously — long TWAPs expose to direction risk) |
| "Estimated cost higher than single market close" | Position too small to need TWAP | Skill suggests `hyperliquid close` directly; user can `--force` to TWAP anyway |

## Security Notices

- **Risk level: `advanced`.** This skill issues market orders autonomously across a duration. Dry-run is the default and required first step.
- **Self-custodial.** Private keys never leave the Agentic Wallet TEE. This skill produces order requests; the wallet signs.
- **No external API calls.** All data comes through `hyperliquid-plugin` and the Hyperliquid endpoints it uses. No third-party signal services, no telemetry.
- **Maximum amount limits** are enforced per-slice via `--max-slippage-per-slice` (hard cap on per-fill loss) and per-session via the user-set position size (TWAP cannot exceed the existing position; it never opens new exposure).
- **Stop-loss equivalent** is implicit: if 3 consecutive slices fail, the schedule pauses and the user is surfaced — preventing silent drift in a hostile market.
- **No automatic re-entry.** When a TWAP completes or is cancelled, the skill is done. It does not re-open, re-balance, or follow the position into another venue.

## Skill Routing

- For **opening** Hyperliquid positions → use `hyperliquid-plugin` directly (`hyperliquid order`)
- For **single-shot market close** of a small position → use `hyperliquid-plugin` (`hyperliquid close`)
- For **rule-driven exits** (trailing stop, liquidation distance, funding rate) → use a separate exit-discipline skill; this one is a *primitive*, not a strategy
- For **spot exits** on Solana / X Layer → use `onchain-livermore` (the spot sibling of this skill) or `okx-dex-swap` directly
- For **cross-DEX TWAP** beyond Hyperliquid → out of scope for v1.0
- For **Hyperliquid spot trading** → use `hyperliquid-plugin` spot commands

## Known Limitations

- **TWAP can lose to a single market close in a directionally moving market.** If the price is dropping fast and the user is trying to close a long, the average TWAP fill will be worse than an immediate close. The skill warns about this in `simulate` but cannot predict short-term direction.
- **No adaptive slice sizing in v1.0.** Equal slices only. Adaptive (book-aware) sizing is a future version concern.
- **No randomized scheduling.** Predictable, auditable timing is the v1.0 choice.
- **Single position per call.** This v1.0 supports one TWAP per coin at a time. Concurrent TWAPs across multiple positions are supported (one per coin), but a second TWAP on the same coin while one is running will be rejected.
- **HIP-3 perp DEXs not supported in v1.0.** Default Hyperliquid perp DEX only.
- **No backtesting.** Defaults are starting points based on common book-depth assumptions, not historical optimization.

---

*Built as a community submission for the OKX Plugin Store Developer Challenge. Companion to `onchain-livermore` (spot) — same philosophy, different venue. Composes the official `hyperliquid-plugin`; not affiliated with Hyperliquid Labs or OKX.*
