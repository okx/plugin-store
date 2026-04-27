# otto-kol-follow

## 1. Overview

Otto KOL Follow is a Strategy Skill for [OKX Onchain OS](https://web3.okx.com/onchainos) Agentic Wallet that mirrors aggregated sentiment from the top 50 crypto KOLs on Twitter/X into a single Hyperliquid perpetual with automatic take-profit and stop-loss brackets, trading only when the cohort shows statistically meaningful consensus.

Core operations:

- Query Otto's KOL-sentiment feed for a user-specified coin or the top-conviction coin across the cohort
- Gate trades on cohort sample size, consensus strength, and confidence thresholds
- Place a Hyperliquid perpetual via the Hyperliquid Basic Skill with explicit user confirmation
- Attach an atomic TP/SL bracket on the same trade
- Enforce position, leverage, and per-session drawdown caps

Designed for the "follow the consensus" user — if crypto Twitter's smartest accounts are loud and aligned, mirror them for a short window with a tight leash.

Tags: `perpetuals` `hyperliquid` `onchainos` `trading-strategy` `kol-sentiment` `social-signals`

## 2. Prerequisites

- US / geofenced users must verify Hyperliquid eligibility in their jurisdiction
- onchainos CLI ≥ 2.0.0 installed and authenticated (`onchainos wallet status`)
- Hyperliquid Basic Skill installed (`npx skills add okx/plugin-store --skill hyperliquid-plugin`)
- USDC on Arbitrum (chain 42161) to deposit into Hyperliquid
- A small amount of ETH on Arbitrum for gas
- Signing address registered via `hyperliquid-plugin register`
- Python 3.8+ for optional `scripts/bot.py` (stdlib only, no pip dependencies)

## 3. Quick Start

1. **Dry-run first.** Paper mode is the default.

   ```
   otto-kol-follow quickstart
   ```

2. **Fund your Hyperliquid account.**

   ```
   hyperliquid-plugin quickstart
   hyperliquid-plugin deposit --amount 50 --confirm
   ```

3. **Test a consensus pick (dry-run).**

   ```
   otto-kol-follow trade --coin ETH --size-usd 25
   ```

4. **Go live** after reviewing ≥ 10 paper trades:

   ```
   otto-kol-follow trade --coin ETH --size-usd 25 --confirm
   ```

5. **Review open positions.**

   ```
   hyperliquid-plugin positions --show-orders
   ```

## Safety defaults

- Dry-run is the default. `--confirm` required to place a live order.
- Position cap: 10% of Hyperliquid account equity per trade (tunable).
- Leverage cap: 3x (KOL consensus is lagging — tight leash by design).
- Auto stop-loss: 2% | Auto take-profit: 4% (2:1 RR).
- Cohort sample-size filter: rejects trades if fewer than 40 KOLs have spoken in the window.
- Confidence filter: rejects trades below 0.70 confidence.
- Per-session drawdown halt: -15% cumulative stops new trades.

## Trigger phrases

The AI agent will route to Otto KOL Follow on intents like:

> "Follow the KOLs on ETH"
> "What are crypto Twitter's smartest saying about BTC?"
> "Mirror the top KOLs into a perp"
> "Consensus trade on SOL"
> "Give me the KOL conviction trade"

## Data moat

Otto's KOL-sentiment pipeline aggregates the top 50 crypto KOLs on Twitter/X with historical accuracy weighting, mention-volume filters, and cohort-diversity checks. Same production feed that drives Otto AI's Market Alpha Agent on the Virtuals Agent Commerce Protocol.

## Risk

Real on-chain leveraged trading. Capital loss is possible. KOL consensus can be reflexively wrong at market tops and bottoms. Dry-run extensively before committing real USDC.

See [SKILL.md](SKILL.md) for the full agent protocol and safety notices.
