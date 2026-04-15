
# pancakeswap-v3-plugin -- Skill Summary

## Overview
This plugin provides comprehensive DeFi functionality for PancakeSwap V3, the leading decentralized exchange on multiple EVM chains. It enables token swaps through the SmartRouter with optimal price discovery across all fee tiers, concentrated liquidity position management with automatic tick range calculation, and real-time market data queries. The plugin integrates seamlessly with onchainos wallet infrastructure for secure transaction execution and supports BNB Chain, Base, Arbitrum, Ethereum, and Linea networks.

## Usage
Install the plugin using the auto-injected setup commands, ensure your onchainos wallet is connected, then use commands like `pancakeswap-v3 quote`, `pancakeswap-v3 swap`, or `pancakeswap-v3 add-liquidity`. All write operations require the `--confirm` flag for execution.

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
An AI agent should activate this skill when users mention PancakeSwap operations, token swapping, liquidity provision, or when they use trigger phrases like "pancakeswap", "swap on pancake", "add liquidity pancakeswap", or "PancakeSwap V3". It should also activate for general DEX operations on supported chains when PancakeSwap would be the optimal choice.
