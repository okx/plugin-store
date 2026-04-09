# pancakeswap-v2 — Skill Summary

## Overview
This skill enables swapping tokens and providing full-range liquidity on PancakeSwap V2 (xyk AMM) on BSC and Base. It supports token swaps, adding/removing liquidity, and querying pair and LP data.

## Commands

| Command | Description |
|---------|-------------|
| `quote` | Get expected swap output and price impact (read-only) |
| `swap` | Swap tokens via PancakeSwap V2 router |
| `add-liquidity` | Add liquidity to a V2 pair and receive LP tokens |
| `remove-liquidity` | Remove liquidity by burning LP tokens |
| `get-pair` | Look up pair contract address for two tokens (read-only) |
| `get-reserves` | Get current reserves for a V2 pair (read-only) |
| `lp-balance` | Check LP token balance for a wallet (read-only) |

## Triggers
Activate when users want to swap on PancakeSwap V2, add/remove liquidity on PancakeSwap, check pair reserves, or view LP balances. Also triggered by: "swap on pancakeswap v2", "pancake swap", "pcs v2 swap", "add liquidity pancakeswap", "remove liquidity pancake", "pancake amm", "pancakeswap v2 quote".
