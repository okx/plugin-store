
# pancakeswap -- Skill Summary

## Overview
This skill enables interaction with PancakeSwap V3, the leading decentralized exchange on BNB Chain, Base, and Arbitrum. It provides comprehensive functionality for token swapping using the SmartRouter for optimal pricing, and concentrated liquidity management through the NonfungiblePositionManager. The skill includes safety features like slippage protection, balance validation, and automatic tick range calculation for liquidity positions.

## Usage
Install the pancakeswap binary and ensure your wallet is connected via `onchainos wallet login`. Use natural language commands like "swap WBNB for USDT" or "add liquidity to WETH/USDC pool" to interact with PancakeSwap V3.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get swap quotes without executing transactions |
| `swap` | Execute token swaps via SmartRouter |
| `pools` | List all pools for a token pair across fee tiers |
| `positions` | View active LP positions for a wallet |
| `add-liquidity` | Mint new concentrated liquidity positions |
| `remove-liquidity` | Remove liquidity and collect tokens from positions |

## Triggers
Activate this skill when users want to perform DeFi operations on PancakeSwap V3, including token swaps, liquidity provision, or position management across supported chains. Use trigger phrases like "pancakeswap", "swap tokens", or "add liquidity".
