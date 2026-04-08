
# pancakeswap -- Skill Summary

## Overview
This skill provides comprehensive integration with PancakeSwap V3, enabling token swaps and concentrated liquidity management on BNB Chain and Base networks. It supports the complete DeFi workflow from getting quotes and executing swaps to minting LP positions, managing liquidity, and monitoring pools across all fee tiers (0.01%, 0.05%, 0.25%, 1%).

## Usage
Ensure your wallet is connected via `onchainos wallet login` and the pancakeswap plugin is installed. Use natural language triggers like "swap on pancakeswap", "add liquidity", or "show my positions" to interact with the protocol.

## Commands
| Command | Purpose |
|---------|---------|
| `quote` | Get swap quotes without executing transactions |
| `swap` | Execute token swaps via SmartRouter with slippage protection |
| `pools` | List all pools for a token pair across fee tiers |
| `positions` | View active LP positions for a wallet address |
| `add-liquidity` | Mint new concentrated liquidity positions |
| `remove-liquidity` | Remove liquidity and collect tokens from positions |

## Triggers
Activate this skill when users want to trade tokens on PancakeSwap, manage liquidity positions, or query pool information. Common triggers include "pancakeswap", "swap tokens", "add/remove liquidity", and "my LP positions".
