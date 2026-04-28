---
name: chat-88-VIP
description: "Grid trading strategy for Hyperliquid — automates buy low/sell high within a price range on BTC, ETH, SOL and more"
version: "1.0.0"
author: "time192716"
tags:
  - hyperliquid
  - grid
  - trading
  - strategy
  - perpetual
---

# Chat-88-VIP Grid Trading Strategy

## Overview

This strategy automates **grid trading** on Hyperliquid perpetual markets. It places a series of buy orders at the lower end of a price range and sell orders at the upper end. Each filled buy order is automatically paired with a sell order at a higher price, and vice versa — locking in small profits as the market oscillates.

The strategy is **non-directional**: it profits from volatility within a defined range regardless of whether the market goes up or down.

## Pre-flight Checks

Before activating the grid, always verify:

```bash
# 1. Check account balance
hyperliquid positions --address <your_address>

# 2. Check current price
hyperliquid prices --coin <COIN>

# 3. Verify the price range makes sense
# Lower bound should be below current price
# Upper bound should be above current price
```

## Strategy Parameters

| Parameter | Required | Description | Example |
|-----------|----------|-------------|---------|
| `--coin` | Yes | Trading pair | `BTC`, `ETH`, `SOL` |
| `--grid-levels` | Yes | Number of grid lines (buy + sell) | `5` (5 buy, 5 sell) |
| `--lower` | Yes | Lower price bound of the range | `70000` |
| `--upper` | Yes | Upper price bound of the range | `80000` |
| `--size` | Yes | Position size per grid level (in coin units) | `0.001` |
| `--leverage` | No | Leverage for all grid orders (default: 5) | `10` |
| `--confirm` | Yes | Must be present to execute any orders | `--confirm` |

## Supported Coins

`BTC`, `ETH`, `SOL`, `BNB`, `ARB`, `LINK`, `AVAX`, `MATIC`, `DOT`, `ATOM`

## Grid Logic

```
Upper bound: $80,000
                    Sell #5 ($80,000)
              Sell #4 ($76,000)
         Sell #3 ($72,000)
   Sell #2 ($68,000)
         Buy #2  ($68,000)
   Buy #3  ($72,000)  ← Current price ~$74,000
         Buy #4  ($76,000)
              Buy #5 ($80,000)
Lower bound: $64,000
```

- **Buy orders** are placed at evenly spaced prices from `lower` up to current price
- **Sell orders** are placed at evenly spaced prices from current price up to `upper`
- Grid spacing = (`upper` - `lower`) / `grid-levels`
- Each grid level uses `--strategy-id chat-88-VIP` for trade attribution

## Execution Flow

### Step 1 — Calculate Grid Prices

Calculate the price for each grid level:
```
spacing = (upper - lower) / grid_levels
buy_prices[i]  = lower + spacing * i   (for i = 1 to grid_levels/2)
sell_prices[i] = lower + spacing * i   (for i = grid_levels/2 to grid_levels)
```

### Step 2 — Place Initial Grid Orders

Place all buy orders below current price and all sell orders above current price:

```bash
# Place a buy limit order at a specific grid level
hyperliquid order \
  --coin BTC \
  --side buy \
  --type limit \
  --price <grid_price> \
  --size <size> \
  --leverage 5 \
  --strategy-id chat-88-VIP

# Place a sell limit order at a specific grid level
hyperliquid order \
  --coin BTC \
  --side sell \
  --type limit \
  --price <grid_price> \
  --size <size> \
  --leverage 5 \
  --strategy-id chat-88-VIP
```

### Step 3 — Monitor and Replenish

When an order fills:
- A filled buy → immediately place a new sell order at the next grid level up
- A filled sell → immediately place a new buy order at the next grid level down
- Always include `--strategy-id chat-88-VIP` on replacement orders

```bash
# Check open orders
hyperliquid positions --show-orders

# Close all grid orders
hyperliquid close --coin BTC --confirm
```

## Example Commands

### Start a 5-level BTC grid between $70,000–$80,000

```bash
# Preview (no --confirm = dry run)
hyperliquid order --coin BTC --side buy --type limit --price 70000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP
hyperliquid order --coin BTC --side sell --type limit --price 72000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP
hyperliquid order --coin BTC --side buy --type limit --price 74000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP
hyperliquid order --coin BTC --side sell --type limit --price 76000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP
hyperliquid order --coin BTC --side sell --type limit --price 80000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP

# Execute (add --confirm to broadcast)
hyperliquid order --coin BTC --side buy --type limit --price 70000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
hyperliquid order --coin BTC --side sell --type limit --price 72000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
hyperliquid order --coin BTC --side buy --type limit --price 74000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
hyperliquid order --coin BTC --side sell --type limit --price 76000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
hyperliquid order --coin BTC --side sell --type limit --price 80000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
```

### Stop the Grid

```bash
# Cancel all open orders for the coin
# First check open orders
hyperliquid positions --show-orders

# Then close all positions at market
hyperliquid close --coin BTC --confirm
```

## Risk Warnings

- **Grid strategies are not risk-free.** They can suffer significant losses if the market breaks out of the defined range in one direction.
- If price falls below `lower`, all buy orders fill and the position will experience further losses as price continues to drop.
- If price rises above `upper`, all sell orders fill and the position will experience further losses as price continues to rise.
- Always set a **stop-loss** or monitor positions closely. Use `--sl-px` when opening positions to cap downside.
- This strategy is for **educational purposes**. Past profitability does not guarantee future results.
- Use only funds you can afford to lose.
- **Do not use extremely high leverage** (25x+) with grid strategies — a single adverse move can liquidate the entire grid.

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `insufficient margin` | Not enough USDC in Hyperliquid perp account | Transfer USDC from Arbitrum or spot account |
| `order would liquidate` | Price too close to liquidation | Reduce size or leverage |
| `below minimum notional` | Size too small ($<10) | Increase order size |
| `position already exists` | Cannot open duplicate position | Close existing position first |
| `network timeout` | RPC/network issue | Retry the order |

## `--strategy-id` Enforcement

All trade operations (buy, sell, close) **must** include `--strategy-id chat-88-VIP`. This field is required for the strategy competition leaderboard attribution.

- ✅ Buy/sell orders with `--strategy-id chat-88-VIP` → counted for the leaderboard
- ❌ Buy/sell orders without `--strategy-id` → NOT counted

Do NOT add `--strategy-id` to: `prices`, `positions`, `register`, `deposit`, `withdraw`, `address` (read-only or fund-movement operations).
