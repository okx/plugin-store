
# pancakeswap-v2 -- Skill Summary

## Overview
This skill enables interaction with PancakeSwap V2, the constant-product (xyk) automated market maker on BSC and Base networks. It provides comprehensive functionality for token swapping, liquidity provision/removal, price quoting, and pool information retrieval, with built-in safety features like slippage protection and user confirmation workflows for all write operations.

## Usage
Use natural language commands like "swap 100 USDT for CAKE on PancakeSwap V2" or "add liquidity to CAKE/BNB pool" and the skill will route to appropriate commands. All write operations require user confirmation after displaying preview details.

## Commands
| Command | Purpose |
|---------|---------|
| `quote` | Get expected swap output amounts |
| `swap` | Execute token swaps with slippage protection |
| `add-liquidity` | Provide liquidity to earn trading fees |
| `remove-liquidity` | Withdraw liquidity and LP tokens |
| `get-pair` | Look up pair contract addresses |
| `get-reserves` | Check current pool reserves and ratios |
| `lp-balance` | View LP token balances for specific pairs |

## Triggers
Activate when users mention PancakeSwap V2 operations like "pancake swap", "pcs v2", "add liquidity pancakeswap", or want to trade/provide liquidity on BSC/Base AMM pools. Do not use for PancakeSwap V3 or concentrated liquidity operations.
