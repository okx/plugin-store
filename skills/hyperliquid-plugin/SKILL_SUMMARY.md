
# hyperliquid-plugin -- Skill Summary

## Overview
This plugin provides comprehensive trading capabilities for Hyperliquid, a high-performance on-chain perpetuals exchange built on its own L1 blockchain. It enables users to manage positions, place orders with advanced bracket features, check prices, handle fund transfers, and deposit USDC from Arbitrum, all while maintaining CEX-like speed with full on-chain settlement in USDC.

## Usage
First run `hyperliquid register` to set up your signing address for trading operations. Use `hyperliquid positions` to check your current positions and `hyperliquid order --coin BTC --side buy --size 0.01 --confirm` to place orders.

## Commands
- `hyperliquid positions [--address] [--show-orders]` - Check open perpetual positions and account summary
- `hyperliquid prices [--coin]` - Get current market mid prices for all or specific markets
- `hyperliquid order --coin --side --size [--type] [--price] [--leverage] [--sl-px] [--tp-px] [--confirm]` - Place market/limit orders with optional TP/SL brackets
- `hyperliquid close --coin [--size] [--confirm]` - Market-close open positions
- `hyperliquid tpsl --coin [--sl-px] [--tp-px] [--size] [--confirm]` - Set stop-loss/take-profit on existing positions
- `hyperliquid cancel --coin --order-id [--confirm]` - Cancel open orders
- `hyperliquid register` - Set up signing address for trading operations
- `hyperliquid transfer --direction --amount [--confirm]` - Transfer funds between perp and spot accounts
- `hyperliquid deposit --amount [--confirm]` - Deposit USDC from Arbitrum to Hyperliquid

## Triggers
Activate this skill when users mention trading on Hyperliquid, opening positions, checking Hyperliquid prices, placing or canceling orders, setting stop-losses or take-profits, or managing funds on the Hyperliquid DEX. The plugin handles both English and Chinese trading terminology.
