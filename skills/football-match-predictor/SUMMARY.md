## Overview

Football Match Predictor is a conservative strategy skill for Polymarket football
and soccer event markets. It searches for relevant match markets, filters out
thin or wide-spread books, converts market prices into implied probabilities,
and prepares small, capped trades only after explicit user confirmation.

The skill is designed for OKX Onchain OS strategy workflows and delegates all
market access and execution to `polymarket-plugin`. It does not custody funds,
does not use private keys, and does not claim guaranteed prediction accuracy.

## Prerequisites

- `polymarket-plugin` installed from `okx/plugin-store`.
- Polymarket access allowed in the user's region.
- Onchain OS wallet configured before live trading.
- Sufficient Polymarket balance for the planned maximum loss.

## Quick Start

1. Ask for a team, match, league, date, or football slate.
2. Search active Polymarket football/soccer markets through
   `polymarket-plugin`.
3. Inspect market details, liquidity, spread, close time, and resolution rules.
4. Present an analysis-only or dry-run trade plan with maximum loss.
5. Execute only after explicit user confirmation.

## Best Use Cases

- Find liquid football match markets on Polymarket.
- Compare implied probabilities across match outcomes.
- Build a small, risk-capped trade plan for a single match or slate.
- Execute confirmed trades with a stable strategy id for attribution.

## Risk Controls

- Defaults to analysis or dry-run before live execution.
- Requires explicit confirmation before any buy or sell command.
- Caps per-market and session exposure.
- Avoids markets with low liquidity, stale timing, or wide spreads.
- Shows maximum loss before execution.
