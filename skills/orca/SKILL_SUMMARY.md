
# orca -- Skill Summary

## Overview
The orca skill provides access to Orca's concentrated liquidity AMM on Solana, enabling users to swap tokens, query pool information, and get swap quotes through the Whirlpools CLMM program. It includes comprehensive safety features like security scanning, price impact checks, and confirmation workflows to protect users during trading operations.

## Usage
Use this skill to interact with Orca DEX by querying pools with `get-pools`, getting swap quotes with `get-quote`, or executing swaps with the `swap` command. All swap operations require user confirmation and include built-in safety checks.

## Commands
| Command | Description |
|---------|-------------|
| `get-pools` | Query Whirlpool pools for a token pair, sorted by TVL |
| `get-quote` | Calculate estimated swap output for a given input amount |
| `swap` | Execute token swap with safety checks and user confirmation |

## Triggers
Activate this skill when users want to swap tokens on Solana, query Orca liquidity pools, or get trading quotes. Common trigger phrases include "swap on orca", "orca pools", "get swap quote", and "whirlpool swap".
