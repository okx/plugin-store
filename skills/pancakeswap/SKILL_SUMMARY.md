
# pancakeswap -- Skill Summary

## Overview
The PancakeSwap skill enables token swapping and concentrated liquidity management on PancakeSwap V3, supporting both BNB Chain (BSC) and Base networks. It provides comprehensive DEX functionality including real-time price quotes, token swaps via SmartRouter, liquidity pool discovery, LP position management, and automated best-price routing across multiple fee tiers. The skill integrates with OKX wallet infrastructure for secure transaction execution and includes safety features like slippage protection and mandatory user confirmations for all write operations.

## Usage
Connect your wallet using `onchainos wallet login`, then use trigger phrases like "swap on pancakeswap", "add liquidity", or "my positions" to interact with the DEX. All write operations require explicit user confirmation before broadcasting transactions.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get swap quotes without executing transactions |
| `swap` | Execute token swaps via SmartRouter |
| `pools` | List available pools for token pairs |
| `positions` | View LP positions for a wallet |
| `add-liquidity` | Mint new concentrated liquidity positions |
| `remove-liquidity` | Remove liquidity and collect tokens |

## Triggers
Activate this skill when users want to trade tokens, provide liquidity, or manage LP positions on PancakeSwap V3. Trigger phrases include "pancakeswap", "swap on pancake", "add liquidity pancakeswap", and "my pancakeswap LP".
