
# quickswap-dex -- Skill Summary

## Overview
This skill provides comprehensive interaction with QuickSwap V2 AMM on Polygon (chain 137), enabling token swaps, liquidity management, and real-time market data queries. It supports native MATIC, wrapped tokens, and major stablecoins with automatic routing through WMATIC for optimal swap paths. The skill includes safety features like dry-run previews, user confirmation requirements, and built-in slippage protection.

## Usage
Install the binary and use commands like `quickswap-dex quote` for price queries or `quickswap-dex swap` for token exchanges. Always use `--dry-run` first for transactions and confirm with users before executing on-chain operations.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get expected swap output (read-only) |
| `swap` | Execute token swaps with confirmation |
| `add-liquidity` | Add liquidity to pools |
| `remove-liquidity` | Remove liquidity from pools |
| `get-pair` | Look up pair contract addresses |
| `get-price` | Get on-chain token prices |
| `get-reserves` | Get current pool reserves |

## Triggers
Activate when users want to swap tokens on QuickSwap, manage liquidity positions, or query Polygon DEX prices. Trigger phrases include "swap on quickswap", "quickswap liquidity", "quickswap polygon", and "quickswap price".
