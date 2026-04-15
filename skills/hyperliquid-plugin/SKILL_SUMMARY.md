
# hyperliquid-plugin -- Skill Summary

## Overview
This plugin provides comprehensive trading capabilities for Hyperliquid, a high-performance on-chain perpetuals exchange built on its own L1 blockchain. It enables users to check positions and balances, place sophisticated orders with stop-loss/take-profit brackets, manage existing positions, get real-time market prices, and handle cross-chain fund transfers between Arbitrum and Hyperliquid accounts.

## Usage
First run `hyperliquid register` to set up your signing address for trading operations. Then use commands like `hyperliquid positions` to check your portfolio or `hyperliquid order --coin BTC --side buy --size 0.01 --confirm` to place orders.

## Commands
- `positions` - Check open perpetual positions and account summary
- `prices` - Get current market prices for all or specific coins
- `order` - Place market/limit orders with optional TP/SL brackets
- `close` - Market-close existing positions
- `tpsl` - Set stop-loss/take-profit on open positions
- `cancel` - Cancel open orders by order ID
- `register` - Set up signing address for trading (one-time setup)
- `deposit` - Transfer USDC from Arbitrum to Hyperliquid
- `withdraw` - Transfer USDC from Hyperliquid back to Arbitrum
- `transfer` - Move funds between perp and spot accounts
- `spot-*` - Spot trading commands (balances, prices, orders)

## Triggers
Activate this skill when users mention Hyperliquid trading, perpetuals, checking positions, placing orders, or managing funds on the Hyperliquid DEX. Common trigger phrases include "trade on Hyperliquid", "check my HL positions", "Hyperliquid long/short", or "HL stop loss/take profit".
