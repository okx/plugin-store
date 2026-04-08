
# hyperliquid -- Skill Summary

## Overview
This skill enables trading perpetual futures on Hyperliquid, a high-performance on-chain derivatives exchange built on its own L1 blockchain. It provides read-only position monitoring and market data access, plus authenticated order placement and cancellation through cryptographic signing. All operations use USDC margin and settle on Hyperliquid L1 with CEX-like speed but full on-chain transparency.

## Usage
Run `hyperliquid positions` to check your current positions and `hyperliquid prices` for market data. For trading, use `hyperliquid order` without `--confirm` to preview, then add `--confirm` to execute after cryptographic signing.

## Commands
| Command | Purpose |
|---------|---------|
| `hyperliquid positions` | View open positions, PnL, and margin usage |
| `hyperliquid positions --address <addr>` | Check positions for specific address |
| `hyperliquid prices` | Get all market mid prices |
| `hyperliquid prices --market <coin>` | Get price for specific market |
| `hyperliquid order --coin <coin> --side <buy/sell> --size <amount> --type <market/limit>` | Place perpetual order (add `--confirm` to execute) |
| `hyperliquid cancel --coin <coin> --order-id <id>` | Cancel open order (add `--confirm` to execute) |

## Triggers
Activate when users want to trade perpetuals on Hyperliquid, check their derivative positions, get perp market prices, or manage leveraged long/short positions on Hyperliquid L1. Also trigger for phrases like "Hyperliquid perps", "HL order", or mentions of trading specific coins with leverage on Hyperliquid.
