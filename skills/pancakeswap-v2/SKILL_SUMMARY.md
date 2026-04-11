
# pancakeswap-v2 -- Skill Summary

## Overview
This skill enables interaction with PancakeSwap V2's constant-product AMM pools across BSC, Base, and Arbitrum networks. It provides comprehensive functionality for token swapping, liquidity management, and pool information retrieval, with built-in safety features like dry-run previews and explicit user confirmation for all write operations.

## Usage
Use onchainos wallet authentication before any write operations. Run commands with `--dry-run` first to preview transactions, then execute after user confirmation.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get expected swap output amounts |
| `swap` | Execute token swaps with slippage protection |
| `add-liquidity` | Provide liquidity to earn LP tokens |
| `remove-liquidity` | Withdraw liquidity by burning LP tokens |
| `get-pair` | Look up pair contract addresses |
| `get-reserves` | Check current pool reserves |
| `lp-balance` | View LP token balances |

## Triggers
Activate when users want to swap tokens on PancakeSwap V2, add/remove liquidity to earn fees, or check pool information and prices. Use for V2 xyk pools only, not V3 concentrated liquidity.
