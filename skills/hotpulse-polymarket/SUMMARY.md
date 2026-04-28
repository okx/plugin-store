# hotpulse-polymarket

## Overview

HotPulse Polymarket is a strategy plugin that scans active Polymarket markets, selects one clear high-attention opportunity, and routes execution through the official `polymarket-plugin` with `--strategy-id hotpulse-polymarket` for attribution.

Core operations:

- Browse hot or breaking Polymarket markets
- Inspect one candidate market before recommending a side
- Preview the exact trade before execution
- Execute only through `polymarket-plugin` with strategy attribution

Tags: `polymarket` `prediction-market` `strategy` `event-trading` `hot-markets`

## Prerequisites

- Polymarket access is region restricted; run `polymarket-plugin check-access` before any trading recommendation
- Supported venue: Polymarket on Polygon through `polymarket-plugin`
- Required tools: `polymarket-plugin` installed and an authenticated onchainos wallet when trading
- The user must provide a budget and risk style
- Every trading command must include `--strategy-id hotpulse-polymarket`

## Quick Start

1. **Check access and wallet readiness**: Run `polymarket-plugin check-access`, then `polymarket-plugin quickstart` or `polymarket-plugin balance`.
2. **Scan hot markets**: Run `polymarket-plugin list-markets --breaking --limit 10` and shortlist only clear, liquid, active markets.
3. **Inspect one candidate**: Run `polymarket-plugin get-market --market-id <slug-or-condition-id>` and choose one side only if the setup is explainable.
4. **Preview before execution**: Run `polymarket-plugin buy ... --dry-run --strategy-id hotpulse-polymarket` and show the resolved order to the user.
5. **Execute only after confirmation**: Run the final `polymarket-plugin buy ... --strategy-id hotpulse-polymarket` command only after explicit user confirmation.
