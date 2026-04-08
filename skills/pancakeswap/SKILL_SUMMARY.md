
# pancakeswap -- Skill Summary

## Overview
This skill enables token swapping and concentrated liquidity management on PancakeSwap V3, the leading decentralized exchange on BNB Chain and Base. It provides comprehensive functionality for trading tokens, adding/removing liquidity positions, getting real-time quotes, and managing V3 NFT positions through PancakeSwap's SmartRouter and NonfungiblePositionManager contracts.

## Usage
Install the plugin via OKX plugin store, connect your wallet with `onchainos wallet login`, then use commands like `pancakeswap swap`, `pancakeswap add-liquidity`, or `pancakeswap quote` to interact with PancakeSwap V3.

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
Activate when users mention PancakeSwap operations like "swap on pancake", "add liquidity pancakeswap", "PCS swap", "pancakeswap pool", or "PancakeSwap V3". Also trigger for general DEX activities on supported chains when PancakeSwap is the preferred platform.
