# otto-alpha-sniper

## 1. Overview

Otto Alpha Sniper is a Strategy Skill for [OKX Onchain OS](https://web3.okx.com/onchainos) Agentic Wallet that turns a user's plain-language trading intent into a single Hyperliquid perpetual trade with automatic take-profit and stop-loss brackets, driven by Otto AI's production alpha signals.

Core operations:

- Fetch live alpha from Otto's signal feed (trending altcoins, KOL sentiment, extreme funding-rates)
- Select the highest-conviction coin + direction for the requested mode
- Place a Hyperliquid perpetual via the Hyperliquid Basic Skill with explicit user confirmation
- Attach an atomic TP/SL bracket on the same trade
- Enforce per-trade, per-session, and per-coin risk caps before anything hits the exchange

Three signal modes (selectable per trade, or auto-blend):

- **Trending** — live altcoin momentum from Otto's trending-altcoins feed (sentiment-weighted, 24h window).
- **KOL-Follow** — aggregated sentiment from the top 50 crypto KOLs on Twitter/X. Mirrors consensus conviction into a short-term perp.
- **Funding-Fade** — captures extreme funding-rate skews. Long the most-shorted asset (negative funding), short the most-longed (positive funding).

Tags: `perpetuals` `hyperliquid` `onchainos` `trading-strategy` `alpha-signals` `kol-sentiment` `funding-rates`

## 2. Prerequisites

- US / geofenced users must verify Hyperliquid eligibility in their jurisdiction
- onchainos CLI ≥ 2.0.0 installed and authenticated (`onchainos wallet status`)
- Hyperliquid Basic Skill installed (`npx skills add okx/plugin-store --skill hyperliquid-plugin`)
- USDC on Arbitrum (chain 42161) to deposit into Hyperliquid
- A small amount of ETH on Arbitrum for gas
- Signing address registered via `hyperliquid-plugin register`
- Python 3.8+ for optional `scripts/bot.py` (stdlib only, no pip dependencies)

## 3. Quick Start

1. **Dry-run first.** Defaults to paper mode — no real orders.

   ```
   otto-alpha-sniper quickstart
   ```

2. **Fund your Hyperliquid account.** From inside Onchain OS:

   ```
   hyperliquid-plugin quickstart
   hyperliquid-plugin deposit --amount 50 --confirm
   ```

3. **Test a signal pick (dry-run).**

   ```
   otto-alpha-sniper trade --mode trending --size-usd 25
   ```

4. **Go live** with explicit `--confirm` (after reviewing at least 10 paper trades):

   ```
   otto-alpha-sniper trade --mode trending --size-usd 25 --confirm
   ```

5. **Review open positions.**

   ```
   hyperliquid-plugin positions --show-orders
   ```

## Safety defaults

- Dry-run is the default. `--confirm` is required to place a live order.
- Position cap: 10% of Hyperliquid account equity per trade (tunable in `scripts/config.py`).
- Auto stop-loss: 2% on market orders.
- Auto take-profit: 4% (2:1 reward:risk by default).
- Leverage cap: 5x for trending mode, 3x for KOL mode, 10x for funding-fade (explicit user override required to exceed).
- Per-session risk budget: strategy halts after -15% drawdown cumulatively.

## Trigger phrases

The AI agent will route to Otto Alpha Sniper on intents like:

> "Otto, what's trending right now and size me a perp trade"
> "Scalp the strongest altcoin"
> "Follow the KOLs on ETH"
> "Fade the funding on the most-longed asset"
> "Open a perp using Otto's alpha signals"
> "Give me a data-driven Hyperliquid trade"

## Data moat

Signals come from Otto AI's production Market Alpha stack: aggregated KOL sentiment across the top 50 crypto Twitter/X accounts, filtered news from 7+ sources, live funding-rate feeds across major CEXs, and real-time trending-altcoin momentum. The same data powers Otto's autonomous trading agents on the Virtuals Agent Commerce Protocol.

## Risk

This is real on-chain trading of perpetual futures with leverage. Capital loss is possible. Dry-run extensively before committing real USDC. Users are responsible for tax reporting, jurisdictional compliance (Hyperliquid geofences some regions), and custody of their Hyperliquid signing key.

See [SKILL.md](SKILL.md) for the full agent protocol and safety notices.
