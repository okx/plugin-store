
# orca -- Skill Summary

## Overview
This skill provides access to Orca's concentrated liquidity AMM on Solana, enabling users to query pool information, get swap quotes, and execute token swaps through the Whirlpools CLMM program. It combines read operations via Orca's REST API with secure write operations through the onchainos framework, including built-in security checks and price impact warnings.

## Usage
Use `orca get-pools` to find available trading pairs, `orca get-quote` to preview swap rates, and `orca swap` to execute trades after confirmation. All swap operations include automatic security scans and require explicit user approval.

## Commands
| Command | Description |
|---------|-------------|
| `orca get-pools` | Query Whirlpool pools for token pairs, sorted by TVL |
| `orca get-quote` | Calculate estimated swap output for given input amount |
| `orca swap` | Execute token swap with safety checks and user confirmation |

## Triggers
Activate when users want to swap tokens on Solana, query Orca liquidity pools, get trading quotes, or need access to concentrated liquidity AMM functionality. Also trigger for phrases like "orca swap", "whirlpool swap", or "swap tokens on solana".
