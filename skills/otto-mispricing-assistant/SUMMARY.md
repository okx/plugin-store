# otto-mispricing-assistant

## 1. Overview

Otto Mispricing Assistant is a Strategy Skill for [OKX Onchain OS](https://web3.okx.com/onchainos) Agentic Wallet that scans active Polymarket prediction markets for statistical divergence between implied odds and Otto AI's real-time news, KOL-sentiment, and funding signals — then surfaces ranked candidates for the user to trade one at a time with explicit confirmation.

Core operations:

- Pull live Otto signals (news-flash, KOL-sentiment, funding-extremes) for macro + crypto topics
- List active Polymarket markets within a configurable resolution window, filtered for liquidity and volume
- Compute a mispricing score per market: `|otto_estimate − implied_prob| × signal_confidence × liquidity_score`
- Present a ranked shortlist (top 5) to the user with per-market reasoning
- Execute a single buy via the Polymarket Basic Skill after the user types "confirm"
- Enforce per-trade and per-session budget caps regardless of user ask

This Skill is a **scanner, not a bot.** It never batch-executes, never trades without the user on a keyboard, and never re-submits failed orders.

Tags: `prediction-markets` `polymarket` `polygon` `onchainos` `trading-strategy` `mispricing` `news-signals`

## 2. Prerequisites

- US and OFAC-sanctioned users are restricted from trading on Polymarket (geofenced)
- Supported chain: Polygon (MATIC). Supported tokens: USDC.e (collateral), POL (gas, or $0 in proxy mode)
- onchainos CLI ≥ 2.0.0 installed and authenticated (`onchainos wallet status`)
- Polymarket Basic Skill installed (`npx skills add okx/plugin-store --skill polymarket-plugin`)
- Polymarket proxy wallet set up (`polymarket-plugin setup-proxy`) — one-time ~$0.01 POL, then gasless trading
- USDC.e funded into the proxy via `polymarket-plugin deposit --amount N`
- Python 3.8+ for optional `scripts/bot.py` scanner (stdlib only, no pip dependencies)

## 3. Quick Start

1. **Dry-run first.** Paper mode is the default.

   ```
   otto-mispricing-assistant quickstart
   ```

2. **Set up Polymarket.**

   ```
   polymarket-plugin check-access
   polymarket-plugin setup-proxy
   polymarket-plugin deposit --amount 50
   ```

3. **Scan for mispricings (dry-run).**

   ```
   otto-mispricing-assistant scan --category crypto
   ```

4. **Pick a market from the ranked list, confirm, and go live** after reviewing paper runs:

   ```
   otto-mispricing-assistant trade --market-id <ID> --amount 10 --confirm
   ```

5. **Review open positions.**

   ```
   polymarket-plugin get-positions
   ```

## Safety defaults

- Dry-run is the default. `--confirm` required for every live trade.
- Per-trade cap: $50 USDC.e. No single trade above this even with user override.
- Per-session cap: $200 USDC.e cumulative.
- Edge threshold: 8 percentage points minimum — smaller edges dominated by noise + fees.
- Resolution window: 14 days — longer-dated markets filtered out.
- Liquidity floor: $5,000 per market. Volume floor: $1,000 (24h).
- Single-market execution only — no batch orders, ever.

## Trigger phrases

The AI agent will route to Otto Mispricing Assistant on intents like:

> "Find me a Polymarket market where the odds don't match the news"
> "Scan for Polymarket mispricings"
> "Where's the Polymarket edge right now?"
> "Show me late-stage Polymarket markets where Otto sees an edge"
> "Polymarket probability vs Otto's view"

## Data moat

Otto's signal feed combines 7+ news sources, top-50 KOL sentiment on Twitter/X, and live funding-rate skews from major CEXs. The mispricing score cross-references these with Polymarket's implied probabilities over a configurable resolution window. Same production pipeline that drives Otto AI's Market Alpha Agent.

## Risk

Prediction markets can resolve against "obvious" signals. Mispricings can persist or widen before closing. A positive-expected-value trade can still lose. Polymarket resolves on specific oracle sources — verify resolution criteria before sizing up. US / OFAC-sanctioned users are geofenced and must not attempt workarounds.

See [SKILL.md](SKILL.md) for the full agent protocol and safety notices.
