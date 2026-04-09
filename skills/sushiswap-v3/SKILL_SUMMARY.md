
# sushiswap-v3 -- Skill Summary

## Overview
This skill enables comprehensive interaction with SushiSwap V3's concentrated liquidity market maker protocol across 38+ EVM chains. It provides token swapping capabilities with automatic fee tier optimization, concentrated liquidity position management through NFT-based positions, and fee collection functionality. The skill integrates directly with QuoterV2 for gas-free quotes and uses the onchainos wallet system for secure transaction execution with user confirmation.

## Usage
Install the binary through the auto-injected setup commands, ensure your onchainos wallet is configured, then use commands like `sushiswap-v3 quote` for price discovery or `sushiswap-v3 swap` for token exchanges. All write operations require user confirmation before execution.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get swap quotes across fee tiers (no gas) |
| `swap` | Execute token swaps with slippage protection |
| `get-pools` | Query pool addresses by token pair |
| `get-positions` | List LP positions owned by wallet |
| `add-liquidity` | Create concentrated liquidity positions |
| `remove-liquidity` | Remove liquidity and collect tokens |
| `collect-fees` | Collect accumulated trading fees |

## Triggers
Activate this skill when users want to trade tokens on SushiSwap V3, manage concentrated liquidity positions, collect LP fees, or query pool information across multiple EVM chains. Use for DeFi operations requiring precise liquidity management and multi-chain DEX functionality.
