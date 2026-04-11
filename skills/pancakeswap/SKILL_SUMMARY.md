
# pancakeswap -- Skill Summary

## Overview
This skill enables token swaps and concentrated liquidity management on PancakeSwap V3 across BNB Chain, Base, and Arbitrum. It provides comprehensive DEX functionality including quote generation, token swaps via SmartRouter, liquidity position management, and pool discovery. The skill uses proper V3 liquidity mathematics for accurate slippage calculations and integrates with both on-chain contracts and TheGraph subgraphs for optimal performance.

## Usage
Install the plugin from OKX plugin store, ensure your wallet is connected via `onchainos wallet login`, then use commands like `pancakeswap quote` for price discovery or `pancakeswap swap` for token exchanges. All write operations require explicit user confirmation and use the `--confirm` flag for transaction broadcasting.

## Commands
- `quote` - Get swap quotes without executing transactions
- `swap` - Execute token swaps via SmartRouter with slippage protection
- `pools` - List all pools for a token pair across fee tiers
- `positions` - View active LP positions for a wallet address
- `add-liquidity` - Mint new concentrated liquidity positions
- `remove-liquidity` - Remove liquidity and collect tokens from positions

## Triggers
Activate when users want to swap tokens on PancakeSwap, manage liquidity positions, or query pool information on BNB Chain, Base, or Arbitrum. Trigger phrases include "pancakeswap", "swap on pancake", "add liquidity pancakeswap", and "PancakeSwap V3".
