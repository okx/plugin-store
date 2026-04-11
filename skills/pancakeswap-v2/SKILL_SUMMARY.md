
# pancakeswap-v2 -- Skill Summary

## Overview
This skill enables AI agents to interact with PancakeSwap V2 DEX across BSC, Base, and Arbitrum networks. It provides comprehensive functionality for token swaps, liquidity management, and pool information retrieval using the constant product AMM formula. All write operations require user confirmation and integrate with onchainos wallet for secure transaction execution.

## Usage
Install the binary via the auto-injected dependency scripts, ensure onchainos wallet is authenticated, then use commands with appropriate global flags like `--chain` and `--dry-run` for transaction preview.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get expected swap output amounts |
| `swap` | Execute token swaps with slippage protection |
| `add-liquidity` | Add liquidity to token pairs |
| `remove-liquidity` | Remove liquidity from positions |
| `get-pair` | Look up pair contract addresses |
| `get-reserves` | Get current pool reserves |
| `lp-balance` | Check LP token balances |

Global flags: `--chain <id>`, `--dry-run`, `--slippage-bps <n>`, `--deadline-secs <n>`, `--from <address>`, `--rpc-url <url>`

## Triggers
Activate this skill when users mention PancakeSwap V2 operations, token swaps on Pancake, liquidity management, or checking pool information on BSC/Base/Arbitrum. Key phrases include "swap pancakeswap v2", "add/remove liquidity pancake", "pcs v2 quote", and "check pancake pair".
