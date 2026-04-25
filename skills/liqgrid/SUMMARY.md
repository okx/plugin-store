# liqgrid

## Overview

liqgrid turns a trader's one-sentence view on a Hyperliquid perpetual
("BTC 90k–95k, balanced, $500 at 5x") into a deterministic, risk-capped
grid strategy. A compiled TypeScript engine computes the full grid plan
(levels, stop-loss, expected fills, margin required) so the math is
reproducible across runs and models; all on-chain writes flow through
the Hyperliquid basic plugin inside the Agentic Wallet.

Core operations:

- Parse a natural-language range-bound view into a dry-run GridPlan
- Compute deterministic grid levels (log-spaced, tick-aligned, dedupe-safe)
- Compute stop-loss trigger price + worst-case-loss bound
- Execute all limit orders + the stop-loss trigger through the Hyperliquid basic plugin
- Resume, inspect, or close an existing grid across sessions via a stable strategy tag

Tags: `hyperliquid` `grid` `perpetuals` `strategy` `agentic-wallet` `passive-income` `automated-trading` `funding-aware` `backtest` `range-bound`

## How It Works

```
User: "BTC 90k-95k range, $300 at 2x"
   ↓
liqgrid Skill (natural-language parsing)
   ↓
   ├─► hyperliquid-plugin            → mark price + 1h candles
   ├─► api.hyperliquid.xyz/info      → live funding rate (read-only)
   └─► liqgrid binary                → deterministic plan
   ↓
Dry-run plan + 7-day backtest preview:
   • 23 tick-aligned rungs, concentrated near mark
   • funding-tilted ±20% (collects funding as alpha)
   • stop-loss + max-loss bound, hard caps enforced
   • realized PnL / max DD / fills on past 7 days
   ↓
User: "go live"
   ↓
hyperliquid-plugin order ... --strategy-id liqgrid1
hyperliquid-plugin tpsl  ... --strategy-id liqgrid1
   ↓
Agentic Wallet TEE signing → on-chain fills
```

Five steps, deterministic at each layer. The compiled binary makes the math
reproducible (same inputs → same plan, byte-identical); the agent layer
handles natural-language parsing and orchestration; all on-chain writes flow
through `hyperliquid-plugin` so signing stays inside the Agentic Wallet TEE.

Two helper subcommands beyond the core flow:

- **`liqgrid quickstart`** — given just `coin` + `notional` + recent candles,
  the binary derives sensible `(rangeLow, rangeHigh, leverage, riskProfile)`
  from the recent vol regime. Use this when the user doesn't pick a range.
- **`liqgrid optimize`** — sweeps 75 (range × leverage × profile) combinations
  on past 30 days, ranks by Calmar score (`realizedPnl / max(maxDD, 1)`),
  returns the top N. Use this to compare hand-picked params to the
  historically-best ones.

## Prerequisites

- No IP restrictions beyond those imposed by Hyperliquid itself
- Supported chain: Hyperliquid L1 (perpetual DEX)
- Supported instruments: liquid Hyperliquid perpetuals (BTC-PERP, ETH-PERP, SOL-PERP, etc.)
- onchainos CLI installed and Agentic Wallet unlocked
- Hyperliquid basic plugin installed: `npx skills add okx/plugin-store --skill hyperliquid-plugin`
- A funded Hyperliquid perp account with ≥ 20 USDC collateral for the chosen notional and leverage

## Quick Start

1. **Describe your range-bound view in natural language**
   Tell the agent what you expect the market to do. For example:
   "plan a balanced ETH grid between 3200 and 3400, $500 at 2x". liqgrid
   starts every session in dry-run by default — nothing is placed yet.

2. **Review the dry-run plan**
   The agent fetches live market metadata and recent hourly candles
   through the Hyperliquid basic plugin, runs the bundled `liqgrid plan`
   binary, and returns a full breakdown: grid count, total notional vs.
   margin required (they differ — margin = notional / leverage), leverage,
   stop-loss trigger price, expected fills/day, approximate liquidation
   buffer, and any safety warnings. Each plan has a stable `planHash` so
   you can reference it later; same inputs always produce the same hash.

3. **Go live only after explicit confirmation**
   Reply "go live" (or "place real orders") to execute. The agent places
   limit orders and the stop-loss trigger through the Hyperliquid basic
   plugin with `--strategy-id liqgrid1` so every fill is attributable to
   this Skill on the Plugin Store leaderboard. If any plan warning
   would allow a loss above the 30% of-notional threshold, the agent
   quotes the warning verbatim and waits for "yes, I'm OK with that"
   before continuing.

4. **Monitor, resume, or close**
   Ask "how is my grid doing" for filled count and realized/unrealized PnL.
   Grids are identified by their strategy tag, so you can resume a grid
   from a new session — liqgrid reads open orders back from Hyperliquid
   and reattaches without any local state. Say "close" to cancel all
   liqgrid-tagged orders and, optionally, flatten the residual position
   with a reduce-only market order.
