# Chat-88-VIP — Grid Trading Strategy

## Overview

Chat-88-VIP is a **grid trading strategy plugin** for Hyperliquid perpetuals. It automates the placement of buy and sell limit orders at evenly spaced price levels within a user-defined range, profiting from market oscillations without requiring directional bias.

## Prerequisites

- Hyperliquid Plugin installed (`hyperliquid-plugin` ^0.3.9)
- USDC on Hyperliquid L1 (deposit from Arbitrum first)
- Basic understanding of grid trading mechanics

## Quick Start

### 1. Deposit USDC to Hyperliquid

```bash
hyperliquid deposit --amount <AMOUNT> --confirm
```

### 2. Check Current Price

```bash
hyperliquid prices --coin BTC
```

### 3. Set Up Grid (Example: BTC, 5 levels, $70k–$80k, 0.001 BTC per level, 5x leverage)

```bash
hyperliquid order --coin BTC --side buy --type limit --price 70000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
hyperliquid order --coin BTC --side sell --type limit --price 72000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
hyperliquid order --coin BTC --side buy --type limit --price 74000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
hyperliquid order --coin BTC --side sell --type limit --price 76000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
hyperliquid order --coin BTC --side sell --type limit --price 80000 --size 0.001 --leverage 5 --strategy-id chat-88-VIP --confirm
```

### 4. Stop the Grid

```bash
hyperliquid close --coin BTC --confirm
```

## Key Parameters

| Parameter | Description |
|-----------|-------------|
| `coin` | Trading pair (BTC, ETH, SOL, etc.) |
| `grid-levels` | Number of buy + sell grid lines |
| `lower` | Lower price bound |
| `upper` | Upper price bound |
| `size` | Position size per level (coin units) |
| `leverage` | Leverage (default: 5) |

## Supported Markets

BTC, ETH, SOL, BNB, ARB, LINK, AVAX, MATIC, DOT, ATOM

## Risk Level

**HIGH** — Grid strategies are susceptible to liquidation during strong directional moves. Never use extreme leverage. Monitor positions actively.

## Attribution

All trade operations use `--strategy-id chat-88-VIP` for leaderboard tracking in the Plugin Store DApp 热度大赛.
