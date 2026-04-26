# pmclock

## Overview

pmclock is a deterministic 5-minute crypto Up/Down arbitrage scanner
for Polymarket. It pulls live BTC/ETH/SOL spot prices from Binance,
computes the GBM-implied probability that the underlying closes above
each market's strike at resolution, compares to the current PM YES/NO
ask, and surfaces decisions only when the **net edge ≥ 1.5%** (after
expected fees). All on-chain writes flow through `polymarket-plugin`
with `--strategy-id pmclock` for leaderboard attribution.

Core operations:

- Scan the next batch of 5-minute crypto Up/Down markets via `polymarket-plugin list-5m`
- Fetch live CEX spot price + recent realized vol from Binance public ticker
- Compute GBM-implied probability and Kelly-fractional sizing per market in a deterministic compiled binary
- Place maker-style limit buys (`--post-only --expires`) via polymarket-plugin only when edge clears the 1.5% threshold
- Auto-redeem winning outcome tokens after 5-minute resolution
- 7 structured rejection reasons (`expired`, `too_far_out`, `low_liquidity`, `edge_below_threshold`, `yes_no_ask_invalid`, `insufficient_budget`, `unknown_direction`) so every skipped market is auditable

Tags: `polymarket` `prediction-market` `arbitrage` `5min` `latency-arb` `crypto` `automated-trading` `passive-income` `agentic-wallet` `strategy`

## Prerequisites

- Polymarket's Terms of Service restrict residents of the United States, France, Singapore, and several other jurisdictions from trading — enforced in pre-flight via `polymarket-plugin check-access`
- Supported chain: Polygon (Polymarket CLOB)
- Supported collateral: USDC.e (bridged USDC on Polygon, `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174`), not native USDC
- Supported coins: BTC, ETH, SOL (any coin Polymarket lists 5-minute Up/Down markets for)
- onchainos CLI installed and Agentic Wallet unlocked
- `polymarket-plugin` installed: `npx skills add okx/plugin-store --skill polymarket-plugin`
- A funded wallet with USDC.e on Polygon (POLY_PROXY mode strongly recommended for gasless trades; deposit via `polymarket-plugin deposit`)
- Public internet access to `api.binance.com` and `gamma-api.polymarket.com` (declared in `plugin.yaml` `api_calls`)

## Quick Start

1. **Tell the agent which coin and how much to risk**
   For example: "Scan BTC 5min arb for me, $50 budget, balanced". pmclock
   starts every new scan in dry-run by default — nothing is placed yet.

2. **Review the dry-run scan output**
   The agent fetches live BTC spot from Binance, computes the recent
   realized vol from 1-minute klines, pulls the next batch of 5-minute
   Up/Down markets from polymarket-plugin, and runs the bundled `pmclock
   plan` binary. You'll see: number of decisions vs markets scanned,
   each decision's side / amount / limit price / edge %, the rationale
   ("CEX 99.9% above 78,000 vs PM YES ask 0.40 → edge 38.9%"), and
   aggregated rejection reasons for skipped markets. Each scan has a
   stable `planHash` so the same scan inputs always produce the same
   decisions.

3. **Go live only after explicit confirmation**
   Reply "place it" (or "go live") to execute. For each decision the
   agent runs `polymarket-plugin buy --market-id <id> --outcome
   <yes|no> --amount <usd> --price <0-1> --post-only --expires <ts>
   --strategy-id pmclock --confirm`. `--post-only` keeps the order
   maker-side (rebate, no taker fee), and `--expires` auto-cancels 30
   seconds before market resolution so no manual cleanup is needed.
   Hard caps in the binary ($200 per scan, $20 per market, $50 minimum
   liquidity, 1.5% minimum edge) cannot be overridden from the Skill —
   to trade more, run another scan; to trade riskier, fork and rebuild.

4. **Loop, monitor, and redeem**
   For longer sessions, ask "run pmclock for 1 hour, $50 budget". The
   agent enters `scan-loop` mode, scanning every 30-60 seconds until
   the time bound, the budget cap, or a -5% session loss-stop triggers.
   After each 5-minute market resolves, the agent calls
   `polymarket-plugin redeem --market-id <id>` to claim winning outcome
   tokens. At any time, "stop" cancels the loop and reports session
   PnL. Because pmclock's binary is fully deterministic, the agent can
   replay any past scan offline (with the same `planHash`) for
   debugging.
