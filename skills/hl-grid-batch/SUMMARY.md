# hl-grid-batch

## Overview

`hl-grid-batch` lets users compose a multi-order trading plan on Hyperliquid
in plain language and submit it as a single atomic batch. The AI agent
parses the intent — uniform grid, staged DCA, or staged take-profit ladder —
generates up to 50 limit orders, shows the full ladder for review, and on
user confirmation submits the entire batch through `hyperliquid order-batch`
with one EIP-712 signature.

The skill exists so a trader can say "BTC 在 58000-62000 之间挂 10 个买单网格，
每格 0.001" and get ten resting limit orders placed atomically, instead of
clicking ten times in a UI. Every order in the batch is attributed to this
strategy via `--strategy-id hl-grid-batch` for the OnchainOS Plugin Store
leaderboard.

## Prerequisites

- `onchainos` CLI installed and the Agentic Wallet logged in
  (`onchainos wallet status` → `loggedIn: true`).
- `hyperliquid-plugin` v0.3.9 or later installed via
  `npx skills add okx/plugin-store --skill hyperliquid-plugin`. The
  `order-batch` command is required and only ships in v0.3.9+.
- Hyperliquid signing address registered (`hyperliquid register`) and the
  perp account funded with USDC. Each individual order in the batch must
  meet the $10 minimum notional; the full batch margin must fit within
  `withdrawable`.
- Margin = sum of (`size × price / leverage`) across all entry orders.
  Reduce-only exit orders do not consume new margin.

## Quick Start

1. Install this skill:
   ```
   npx skills add okx/plugin-store --skill hl-grid-batch
   ```
2. Tell your AI agent the trading plan, in plain language. Examples:
   - "Build a BTC buy grid between 58000 and 62000 with 10 levels of 0.001 each."
   - "ETH 跌到 3000 分 5 笔加仓，每笔 0.05，全部限价。"
   - "Stage take-profit on my SOL long: 10 sells from 200 to 250, 0.1 SOL each, reduce-only."
3. Review the full ladder the agent shows you (every price and size,
   total notional, total margin, batch summary).
4. Confirm. The agent invokes `hyperliquid order-batch --confirm` once;
   one signature places all orders. Each resting order generates its own
   attribution call under `hl-grid-batch`.
5. Manage the batch later: "cancel all my BTC limit orders" →
   `hyperliquid cancel-batch` (the underlying `hyperliquid-plugin`
   command), or "show my open orders" → `hyperliquid orders`.
