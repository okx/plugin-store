# safeedge-polymarket

## Overview

SafeEdge Polymarket is a conservative strategy plugin that filters for clearer Polymarket setups, uses calmer sizing, and routes execution through the official `polymarket-plugin` with `--strategy-id safeedge-polymarket` for attribution.

Core operations:

- Browse active Polymarket event markets
- Inspect candidates for liquidity, clarity, and order acceptance
- Preview conservative orders before execution
- Execute only through `polymarket-plugin` with strategy attribution

Tags: `polymarket` `risk-control` `strategy` `conservative-trading` `event-markets`

## Prerequisites

- Polymarket access is region restricted; run `polymarket-plugin check-access` before any trading recommendation
- Supported venue: Polymarket on Polygon through `polymarket-plugin`
- Required tools: `polymarket-plugin` installed and an authenticated onchainos wallet when trading
- The user must provide total budget and risk style
- Every trading command must include `--strategy-id safeedge-polymarket`

## Quick Start

1. **Check access and wallet readiness**: Run `polymarket-plugin check-access`, then `polymarket-plugin quickstart` or `polymarket-plugin balance`.
2. **Browse candidates**: Run `polymarket-plugin list-markets --limit 20` and filter out noisy, illiquid, or unclear markets.
3. **Inspect each candidate**: Run `polymarket-plugin get-market --market-id <slug-or-condition-id>` before recommending a side.
4. **Preview before execution**: Run `polymarket-plugin buy ... --dry-run --strategy-id safeedge-polymarket`.
5. **Execute only after confirmation**: Run the final `polymarket-plugin buy ... --strategy-id safeedge-polymarket` command only after explicit user confirmation.
