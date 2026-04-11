
# pancakeswap-v2 -- Skill Summary

## Overview
This skill enables interaction with PancakeSwap V2's constant-product (xyk) automated market maker across BSC, Base, and Arbitrum One networks. It provides comprehensive functionality for token swapping, liquidity management, and pool data retrieval with built-in safety features including slippage protection, exact-amount approvals, and transaction preview capabilities.

## Usage
Install the binary using the auto-injected installation script, ensure onchainos CLI is authenticated with `onchainos wallet login`, then use commands like `pancakeswap-v2 quote` for price queries or `pancakeswap-v2 swap` for token exchanges. Always run with `--dry-run` first for write operations and confirm transaction details before execution.

## Commands
| Command | Purpose |
|---------|---------|
| `quote` | Get expected swap output amounts |
| `swap` | Exchange tokens with slippage protection |
| `add-liquidity` | Provide liquidity to earn LP tokens |
| `remove-liquidity` | Withdraw liquidity and burn LP tokens |
| `get-pair` | Look up pair contract address |
| `get-reserves` | Check pool reserve balances |
| `lp-balance` | View LP token holdings |

## Triggers
Activate when users want to swap tokens on PancakeSwap V2, add/remove liquidity, get price quotes, or check pool information. Look for phrases like "swap on pancakeswap v2", "add liquidity pancake", "pcs v2 quote", or "check pancake pair".
