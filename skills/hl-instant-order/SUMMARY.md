# hl-instant-order

## Overview

`hl-instant-order` lets users open Hyperliquid perpetual positions in one
sentence. The AI agent parses the user's natural-language intent (coin, side,
size or notional, leverage, optional stop-loss / take-profit), shows a
preview of the exact `hyperliquid` command, and executes it through the
OnchainOS Agentic Wallet only after explicit user confirmation. Stop-loss
and take-profit are placed atomically with the entry as a single OCO
bracket — no second click required.

The skill exists so a trader can say "做空 BTC 5x，止损 70k，止盈 60k" and
get a fully bracketed perp position without touching a UI, while every
trade is attributed to this strategy via `--strategy-id hl-instant-order`
for the OnchainOS Plugin Store leaderboard.

## Prerequisites

- `onchainos` CLI installed and the Agentic Wallet logged in
  (`onchainos wallet login <email>` → `onchainos wallet status` → `loggedIn: true`).
- `hyperliquid-plugin` v0.3.9 or later installed via
  `npx skills add okx/plugin-store --skill hyperliquid-plugin`.
- Hyperliquid signing address registered (`hyperliquid register` once per
  wallet) and the perp account funded — deposit USDC from Arbitrum with
  `hyperliquid deposit --amount <N> --confirm` if needed.
- USDC balance on the Hyperliquid perp account sufficient for the position
  margin (`notional / leverage`). Pre-flight balance check runs
  automatically before each order; insufficient margin aborts the trade
  with a clear tip.

## Quick Start

1. Install this skill:
   ```
   npx skills add okx/plugin-store --skill hl-instant-order
   ```
2. Tell your AI agent what you want, in plain language. Examples:
   - "Open a 5x long on BTC with 200 USDC margin, stop loss at 60000, take profit at 75000."
   - "做空 ETH 3x，1000U notional，止损 3500，止盈 2800"
   - "Long SOL 2x, 50 USDC, no stops."
3. Review the preview the agent shows you (coin, side, size, leverage,
   trigger prices, estimated margin, current mid-price).
4. Confirm. The agent runs the same command with `--confirm` — one EIP-712
   signature opens the entry and arms the SL/TP bracket together.
5. Manage the position later with the same agent: "close my BTC long",
   "tighten BTC stop to 65000", "show my Hyperliquid positions".
