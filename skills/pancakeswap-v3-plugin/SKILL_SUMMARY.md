
# pancakeswap-v3-plugin -- Skill Summary

## Overview
This plugin enables AI agents to interact with PancakeSwap V3, the leading decentralized exchange protocol, providing comprehensive token swapping and concentrated liquidity management capabilities. It supports multi-chain operations across BNB Chain, Base, Arbitrum, Ethereum, and Linea, offering features like real-time price quotes, optimal routing through SmartRouter, LP position management, and portfolio tracking with built-in safety measures including slippage protection and balance validation.

## Usage
Install the plugin and connect your wallet using `onchainos wallet login`, then use commands like `pancakeswap-v3-plugin swap`, `pancakeswap-v3-plugin add-liquidity`, or `pancakeswap-v3-plugin quickstart` to begin trading and managing liquidity positions.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get swap quotes without executing transactions |
| `swap` | Execute token swaps via SmartRouter |
| `pools` | List available pools for token pairs |
| `positions` | View active LP positions for a wallet |
| `add-liquidity` | Mint new concentrated liquidity positions |
| `remove-liquidity` | Remove liquidity and collect tokens from positions |
| `quickstart` | Check wallet status and get onboarding guidance |

## Triggers
An AI agent should activate this skill when users mention PancakeSwap operations like "swap on pancakeswap", "add liquidity pancakeswap", "pancakeswap pool", or need DEX functionality on supported chains. It's specifically designed for PancakeSwap V3 concentrated liquidity operations rather than V2 AMM swaps.
