# hyperliquid-funding-rate-scout

## Overview

Hyperliquid Funding Rate Scout is a strategy skill that identifies high-probability
mean reversion opportunities by scanning perpetual futures markets on Hyperliquid
for statistically significant funding rate imbalances and crowded positioning.

Core operations:

- Scan all active Hyperliquid perpetual markets for extreme funding rate conditions
- Rank signals by deviation severity (Elevated / High / Extreme) using hourly rate thresholds
- Generate structured signal cards with direction, thesis, supporting factors, and risk notes
- Cross-reference funding extremes with smart money positioning and recent price action
- Hand off confirmed trades to `hyperliquid-plugin` with full strategy attribution

Tags: `hyperliquid` `funding-rate` `mean-reversion` `strategy` `perpetuals` `signal` `crowded-positioning`

## Prerequisites

- No IP or region restrictions
- Supported chain: Arbitrum (Hyperliquid perpetuals are settled on Arbitrum)
- Supported markets: All active Hyperliquid perpetual futures pairs
- `onchainos` CLI installed: `npx skills add okx/onchainos-skills`
- `hyperliquid-plugin` installed (required only if user confirms execution):
  `npx skills add okx/plugin-store --skill hyperliquid-plugin`
- No wallet connection required for scanning and signal generation
- A funded wallet with USDC on Arbitrum is required only if proceeding to execution

## Quick Start

1. **Request a scan**: Ask the agent "find me funding rate opportunities" or "what are
   the most extreme funding rates on Hyperliquid right now?" The agent will fetch
   live data across all active perp markets and filter by the default ±50% APR threshold.

2. **Review signal cards**: The agent returns ranked signal cards — each showing the
   asset, direction (LONG or SHORT), conviction level, funding rate, mean reversion
   thesis, supporting factors, and risk notes. Signals are sorted by funding rate
   deviation, highest conviction first.

3. **Optional — filter results**: Narrow the scan by providing an asset filter
   (e.g. "only BTC and ETH"), a custom threshold (e.g. "only show above 100% APR"),
   or a signal count (e.g. "top 3 signals only").

4. **Confirm a trade (optional)**: If you want to act on a signal, tell the agent
   which one. It will collect your position size, leverage, and stop-loss, then
   present a full trade summary for your review. No trade is placed until you
   explicitly confirm.

5. **Execution handoff**: After confirmation, the agent calls `hyperliquid-plugin`
   with `--strategy-id hyperliquid-funding-rate-scout` for leaderboard attribution.
   Autonomous execution is never permitted — every trade requires explicit approval.