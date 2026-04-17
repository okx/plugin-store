
# pendle-plugin -- Skill Summary

## Overview
This plugin enables comprehensive interaction with Pendle Finance's yield tokenization protocol, allowing users to trade fixed-yield Principal Tokens (PT), speculate on floating-yield Yield Tokens (YT), provide liquidity to AMM pools, and mint/redeem tokenized yield positions. The plugin supports four major networks and includes safety features like price impact warnings and preview modes.

## Usage
Install the plugin and run commands through the `pendle` CLI. Most operations support a preview mode (default) to show expected outputs before executing with the `--confirm` flag.

## Commands
- `list-markets` - Browse available Pendle markets with filtering options
- `get-market` - Get detailed market information and APY data
- `get-market-info` - Retrieve PT/YT/SY addresses for a market
- `get-positions` - View user's Pendle positions and balances
- `get-asset-price` - Get current prices for PT/YT/LP tokens
- `buy-pt` - Purchase Principal Tokens for fixed yield
- `sell-pt` - Sell Principal Tokens back to underlying
- `buy-yt` - Purchase Yield Tokens for leveraged yield exposure
- `sell-yt` - Sell Yield Tokens back to underlying
- `add-liquidity` - Provide liquidity to Pendle AMM pools
- `remove-liquidity` - Remove liquidity from AMM positions
- `mint-py` - Mint PT+YT pairs from underlying assets
- `redeem-py` - Redeem PT+YT pairs back to underlying

## Triggers
An AI agent should activate this skill when users mention Pendle operations like "buy PT", "sell YT", "Pendle fixed yield", "add liquidity Pendle", or ask about yield tokenization, fixed yield positions, or Pendle market data.
