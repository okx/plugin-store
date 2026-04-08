
# hyperliquid -- Skill Summary

## Overview
This skill enables trading perpetual futures on Hyperliquid, a high-performance on-chain derivatives exchange built on its own L1 blockchain. Users can check positions, get market prices, place leveraged long/short orders, and cancel existing orders across 100+ perpetual markets including BTC, ETH, SOL and other major cryptocurrencies. All positions are settled in USDC with full on-chain execution while maintaining CEX-like speed and user experience.

## Usage
Run `hyperliquid positions` to check your current perpetual positions, `hyperliquid prices` for market data, and use `hyperliquid order` with `--confirm` to place trades. All write operations use a two-step preview-then-confirm flow for safety.

## Commands
| Command | Purpose | Example |
|---------|---------|---------|
| `positions` | Check open perpetual positions and PnL | `hyperliquid positions --show-orders` |
| `prices` | Get current market mid prices | `hyperliquid prices --market BTC` |
| `order` | Place market or limit perpetual orders | `hyperliquid order --coin BTC --side buy --size 0.01 --type market --confirm` |
| `cancel` | Cancel open orders by ID | `hyperliquid cancel --coin BTC --order-id 91490942 --confirm` |

## Triggers
Activate when users mention trading perpetuals, opening leveraged positions, or checking prices on Hyperliquid - phrases like "Hyperliquid perps", "HL long BTC", "trade on Hyperliquid", or "check my Hyperliquid positions". Also trigger for general perpetual trading requests when context suggests derivatives rather than spot trading.
