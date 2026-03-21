---
name: hyperliquid
description: >-
  Use when the user asks about Hyperliquid, perpetual futures, 'open a long position',
  'short BTC', 'check my perp positions', 'funding rate', 'Hyperliquid orderbook',
  'spot trade on Hyperliquid', 'set leverage', '做多', '做空', '永续合约', '资金费率',
  '杠杆交易', or mentions Hyperliquid DEX, perpetual trading, funding rates, or
  leverage trading. Covers perpetual and spot markets, prices, orderbook, funding
  rates, and trading (buy/sell/cancel). Do NOT use for Polymarket prediction markets.
  Do NOT use for Aave lending.
license: Apache-2.0
metadata:
  author: Plugin Store
  category: "DeFi"
  chain: "Arbitrum,Ethereum"
  version: "0.1.18"
  homepage: "https://github.com/okx/plugin-store"
---

# Hyperliquid Perpetual & Spot Trading CLI

11 commands for perpetual futures and spot trading on Hyperliquid: market data, orderbook, funding rates, and order management.

## Pre-flight Checks

Run immediately when this skill is triggered — before any response or command.

1. **Check binary**: `which dapp-hyperliquid` — if not found, install via `plugin-store install hyperliquid`
2. **Check EVM_PRIVATE_KEY** for trading commands: verify it is set in the environment or `.env` file — if not, remind the user to set it before calling buy/sell/cancel/positions/balances/orders

## Authentication

**Data commands (markets, spot-markets, price, orderbook, funding):** No authentication needed.

**Trading commands (buy, sell, cancel, positions, balances, orders):** Require an EVM wallet private key:

```bash
# Add to .env file in your project directory
EVM_PRIVATE_KEY=0x...
```

The private key is used to sign Hyperliquid L1 actions via EIP-712 typed data signatures.

## Quickstart

```bash
# List all perpetual markets
dapp-hyperliquid markets

# List all spot markets
dapp-hyperliquid spot-markets

# Get BTC mid price
dapp-hyperliquid price BTC

# View BTC orderbook
dapp-hyperliquid orderbook BTC

# Check BTC funding rate
dapp-hyperliquid funding BTC

# Open a long: buy 0.01 BTC at $65000 with 10x leverage
dapp-hyperliquid buy --symbol BTC --size 0.01 --price 65000 --leverage 10

# Check positions
dapp-hyperliquid positions

# Check balances
dapp-hyperliquid balances
```

## Command Index

| # | Command | Auth | Description |
|---|---------|------|-------------|
| 1 | `dapp-hyperliquid markets` | No | List perpetual markets (price, leverage) |
| 2 | `dapp-hyperliquid spot-markets` | No | List spot markets |
| 3 | `dapp-hyperliquid price <symbol>` | No | Real-time mid price for a symbol |
| 4 | `dapp-hyperliquid orderbook <symbol>` | No | L2 order book snapshot |
| 5 | `dapp-hyperliquid funding <symbol>` | No | Current and historical funding rates |
| 6 | `dapp-hyperliquid buy --symbol <s> --size <n> --price <p> [--leverage <l>]` | Yes | Buy (long perp or spot buy) |
| 7 | `dapp-hyperliquid sell --symbol <s> --size <n> --price <p>` | Yes | Sell (short perp or spot sell) |
| 8 | `dapp-hyperliquid cancel --symbol <s> --order-id <oid>` | Yes | Cancel an open order |
| 9 | `dapp-hyperliquid positions` | Yes | View perpetual positions |
| 10 | `dapp-hyperliquid balances` | Yes | View USDC margin and spot balances |
| 11 | `dapp-hyperliquid orders [--symbol <s>]` | Yes | List open orders |

## Operation Flow

### Intent: Research and Trade

```
1. dapp-hyperliquid markets              → browse available perpetual markets
2. dapp-hyperliquid funding BTC          → check funding rates
3. dapp-hyperliquid price BTC            → get current mid price
4. dapp-hyperliquid orderbook BTC        → check spread and liquidity
5. dapp-hyperliquid buy --symbol BTC --size 0.01 --price 65000 --leverage 10
6. dapp-hyperliquid positions            → verify position opened
```

### Intent: Position Management

```
1. dapp-hyperliquid positions            → view open positions
2. dapp-hyperliquid orders               → list pending orders
3. dapp-hyperliquid cancel --symbol BTC --order-id 123456
4. dapp-hyperliquid sell --symbol BTC --size 0.01 --price 66000
```

### Intent: Spot Trading

```
1. dapp-hyperliquid spot-markets         → browse spot pairs
2. dapp-hyperliquid price PURR           → check token price
3. dapp-hyperliquid buy --symbol PURR --size 100 --price 0.50
```

## Key Concepts

- **Perpetual Futures**: Contracts with no expiry — funding payments keep price aligned with index
- **Funding Rate**: Hourly payment between longs/shorts. Positive = longs pay shorts
- **Leverage**: Cross margin by default (all positions share the same pool). BTC up to 50x
- **szDecimals**: Each asset has size precision — orders with wrong decimal places are rejected
- **USDC Margin**: All perp positions are margined in USDC on Hyperliquid L1

## Risk Warning

- Perpetual futures trading carries significant risk of loss
- Higher leverage means closer liquidation price — always monitor your liquidation price
- Always check `balances` before opening positions to ensure sufficient margin
- Use `positions` regularly to monitor unrealized PnL

## Response Guidelines

- Before any buy/sell, show symbol, size, price, leverage, and ask for confirmation
- Always check `EVM_PRIVATE_KEY` is set before trading commands
- After any action, suggest 2–3 natural follow-ups
- Support both English and Chinese — respond in the user's language

> Full command reference, return fields, decimal normalization rules, edge cases, and environment variables: see `references/command-reference.md`
