
# pancakeswap -- Skill Summary

## Overview
This skill enables token swapping and concentrated liquidity management on PancakeSwap V3, the leading DEX on BNB Chain and Base. It provides comprehensive functionality for trading tokens via the SmartRouter, managing LP positions with precise tick ranges, and querying pool data across all fee tiers. The skill integrates with onchainos wallet infrastructure for secure transaction execution and includes safety features like slippage protection and multi-step confirmation flows.

## Usage
Install the plugin via OKX plugin store, ensure onchainos wallet is connected, then use commands like `pancakeswap swap`, `pancakeswap add-liquidity`, or `pancakeswap quote` to interact with PancakeSwap V3 pools.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get swap quotes without executing transactions |
| `swap` | Execute token swaps via SmartRouter |
| `pools` | List all pools for a token pair across fee tiers |
| `positions` | View LP positions for a wallet address |
| `add-liquidity` | Mint new concentrated liquidity positions |
| `remove-liquidity` | Remove liquidity and collect tokens from positions |

## Triggers
Activate when users mention "pancakeswap", "swap on pancake", "PCS swap", "add liquidity pancakeswap", "remove liquidity pancakeswap", "pancakeswap pool", or "PancakeSwap V3". Also trigger for general DEX operations on BNB Chain or Base networks.
