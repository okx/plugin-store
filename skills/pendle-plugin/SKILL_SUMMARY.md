
# pendle-plugin -- Skill Summary

## Overview
The pendle-plugin enables AI agents to interact with Pendle Finance's yield tokenization protocol. It allows users to split yield-bearing assets into Principal Tokens (PT) for fixed yields and Yield Tokens (YT) for variable yield exposure, manage liquidity positions in Pendle's AMM pools, and track portfolio positions across multiple chains. The plugin handles complex DeFi operations like token approvals, slippage protection, and cross-asset routing through Pendle's SDK integration.

## Usage
Install via the auto-injected setup commands, then use `pendle-plugin quickstart` to check onboarding status. Execute operations in preview mode first (no flags) to see expected outcomes, then add `--confirm` to broadcast transactions.

## Commands
- `quickstart` - Check wallet onboarding status and get started
- `list-markets` - Browse available Pendle markets with filtering
- `get-market` - Get detailed market information and APY data
- `get-market-info` - Get PT/YT/SY addresses for a market
- `get-positions` - View current Pendle positions and balances
- `get-asset-price` - Get current prices for PT/YT/LP tokens
- `buy-pt` - Purchase Principal Tokens for fixed yield
- `sell-pt` - Sell Principal Tokens back to underlying
- `buy-yt` - Purchase Yield Tokens for variable yield exposure
- `sell-yt` - Sell Yield Tokens back to underlying
- `add-liquidity` - Provide liquidity to Pendle AMM pools
- `remove-liquidity` - Remove liquidity from Pendle pools
- `mint-py` - Mint PT+YT pairs from underlying assets
- `redeem-py` - Redeem PT+YT pairs back to underlying

## Triggers
An AI agent should activate this skill when users mention Pendle operations like "buy PT", "sell YT", "Pendle fixed yield", "add Pendle liquidity", "mint PT YT", or want to explore yield tokenization strategies. Also trigger for Chinese phrases like "购买PT", "出售YT", or "Pendle固定收益".
