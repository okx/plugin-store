## Overview

Polymarket Starter Kit is a guided first-trade flow on Polymarket for users new to prediction markets. It wraps `polymarket-plugin` with three added safety behaviors: budget protection (refuses if USDC.e < 2× bet size), fixed bet tiers ($1 / $5 / $10 only), and a binary-markets-only filter (no multi-outcome markets). All on-chain operations are delegated to `polymarket-plugin`; this skill adds no new signing logic. Every write call carries `--strategy-id polymarket-starter` so trades are attributed to this strategy on the OKX backend.

## Prerequisites

- `polymarket-plugin` v0.4.10 or later installed (`npx skills add okx/plugin-store --skill polymarket-plugin --yes --global`)
- `onchainos` CLI installed and logged in with a Polygon address (chain 137)
- `onchainos wallet report-plugin-info --help` must succeed (needed for strategy attribution; run `onchainos upgrade` if missing)
- USDC.e on Polygon ≥ 2× your chosen bet tier ($2 for $1 tier, $10 for $5 tier, $20 for $10 tier)
- Accessible region — Polymarket blocks US and OFAC-sanctioned jurisdictions

## Quick Start

1. Confirm wallet login and Polygon address: `onchainos wallet status` then `onchainos wallet addresses --chain 137`
2. Confirm region access: `polymarket-plugin check-access` — must return `accessible: true`
3. Check balance: `polymarket-plugin balance` — fund to ≥ 2× your planned bet size ($2 for $1 tier, etc.) if low
4. Tell the AI agent what you want to bet on (topic or keyword, e.g. "bitcoin", "election", "champions league")
5. The agent discovers markets via `polymarket-plugin list-markets --keyword <topic>`, filters to binary YES/NO only, and presents top 3
6. Pick a market, choose YES or NO, and select an amount: **$1 / $5 / $10** (larger bets → invoke `polymarket-plugin` directly, remembering to keep `--strategy-id polymarket-starter` for attribution)
7. Review the confirmation card showing market, outcome, amount, current price, expected shares, max loss, and max gain — confirm with `yes`
8. Agent runs `polymarket-plugin buy --market-id <id> --outcome <yes|no> --amount <1|5|10> --strategy-id polymarket-starter` and reports fill details
9. Review your new position: `polymarket-plugin get-positions` (read-only, no strategy-id). Sell anytime with `polymarket-plugin sell ... --strategy-id polymarket-starter`, or wait for resolution and `polymarket-plugin redeem ... --strategy-id polymarket-starter`
