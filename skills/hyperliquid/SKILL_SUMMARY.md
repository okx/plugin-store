
# hyperliquid -- Skill Summary

## Overview
This skill provides comprehensive trading capabilities for Hyperliquid, a high-performance on-chain perpetuals exchange built on its own L1 blockchain. It enables users to check positions, get real-time prices, place sophisticated orders with stop-loss and take-profit brackets, manage existing positions, and deposit funds from Arbitrum. All operations use USDC settlement and support both cross and isolated margin trading with leverage up to 100x.

## Usage
Run `hyperliquid register` once to set up your signing address, then use commands like `hyperliquid positions` to check your account or `hyperliquid order --coin BTC --side buy --size 0.01 --confirm` to place trades. All write operations require the `--confirm` flag to execute.

## Commands
| Command | Description |
|---------|-------------|
| `hyperliquid register` | Set up signing address for trading (one-time setup) |
| `hyperliquid positions [--address] [--show-orders]` | Check open positions and account summary |
| `hyperliquid prices [--coin]` | Get current market prices for all or specific coins |
| `hyperliquid order --coin --side --size [--type] [--price] [--sl-px] [--tp-px] [--confirm]` | Place market/limit orders with optional TP/SL brackets |
| `hyperliquid close --coin [--size] [--confirm]` | Market close existing positions |
| `hyperliquid tpsl --coin [--sl-px] [--tp-px] [--confirm]` | Set stop-loss/take-profit on open positions |
| `hyperliquid cancel --coin --order-id [--confirm]` | Cancel open orders |
| `hyperliquid deposit --amount [--confirm]` | Deposit USDC from Arbitrum |

## Triggers
An AI agent should activate this skill when users mention Hyperliquid trading activities, perpetual contracts, or specific phrases like "trade on Hyperliquid," "HL order," "Hyperliquid positions," or "close my HL position." It should also trigger for general derivatives trading requests when Hyperliquid is the preferred platform.
