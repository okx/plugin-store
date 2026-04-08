
# pancakeswap -- Skill Summary

## Overview
This skill enables token swapping and concentrated liquidity management on PancakeSwap V3, the leading decentralized exchange on BNB Chain and Base. It provides comprehensive DEX functionality including real-time quotes, token swaps via SmartRouter, liquidity position minting/burning, pool analytics, and portfolio tracking. All operations integrate with the onchainos wallet system and include safety checks with user confirmation steps for write operations.

## Usage
Install the plugin via OKX plugin store, ensure your onchainos wallet is connected, then use commands like `pancakeswap quote`, `pancakeswap swap`, or `pancakeswap add-liquidity`. All write operations require explicit user confirmation before broadcasting transactions.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get swap quotes without executing transactions |
| `swap` | Execute token swaps via SmartRouter |
| `pools` | List available pools for token pairs |
| `positions` | View LP positions for a wallet address |
| `add-liquidity` | Mint new concentrated liquidity positions |
| `remove-liquidity` | Remove liquidity and collect tokens from positions |

## Triggers
Activate this skill when users mention PancakeSwap operations like "swap on pancake", "add liquidity pancakeswap", "PCS swap", or want to manage V3 concentrated liquidity positions. Do not use for PancakeSwap V2 AMM operations.
