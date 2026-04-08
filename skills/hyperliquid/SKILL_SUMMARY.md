
# hyperliquid -- Skill Summary

## Overview
This skill provides comprehensive trading capabilities for Hyperliquid, a high-performance on-chain perpetuals exchange built on its own L1 blockchain. It enables users to manage positions, execute trades, set risk management orders, and bridge assets, all with CEX-like speed but full on-chain settlement in USDC.

## Usage
Install the binary and register your API wallet once at https://app.hyperliquid.xyz for write operations. Use commands without `--confirm` to preview, then add `--confirm` to execute trades.

## Commands
| Command | Description |
|---------|-------------|
| `hyperliquid positions` | Check open positions and account summary |
| `hyperliquid prices` | Get market mid prices for all or specific coins |
| `hyperliquid order` | Place market/limit orders with optional TP/SL brackets |
| `hyperliquid close` | Market-close existing positions |
| `hyperliquid tpsl` | Set stop-loss/take-profit on open positions |
| `hyperliquid cancel` | Cancel open orders by ID |
| `hyperliquid deposit` | Deposit USDC from Arbitrum to Hyperliquid |

## Triggers
Activate when users mention Hyperliquid trading, perpetual positions, HL orders, or want to trade perps with phrases like "trade on Hyperliquid", "HL long/short", or "Hyperliquid prices". Also trigger for stop-loss/take-profit management and position closing on the platform.
