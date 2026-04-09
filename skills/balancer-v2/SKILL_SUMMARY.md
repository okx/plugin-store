
# balancer-v2 -- Skill Summary

## Overview
This skill provides comprehensive access to Balancer V2, a multi-token automated market maker (AMM) DEX on Ethereum and Arbitrum. It enables users to swap tokens, manage liquidity positions, query pool information, and get trading quotes through Balancer's unified Vault architecture. All operations route through the same Vault contract address across supported chains, providing a consistent interface for DeFi interactions.

## Usage
Install the skill via `npx skills add okx/plugin-store --skill balancer-v2` and ensure onchainos CLI is available. All write operations require explicit user confirmation via the `--confirm` flag after previewing transaction details.

## Commands
| Command | Description |
|---------|-------------|
| `pools` | List top Balancer V2 pools by liquidity |
| `pool-info` | Get detailed pool information (tokens, balances, weights) |
| `quote` | Get swap quotes via on-chain BalancerQueries contract |
| `positions` | View LP positions and BPT holdings for connected wallet |
| `swap` | Execute token swaps through Vault.swap() |
| `join` | Add liquidity via Vault.joinPool() |
| `exit` | Remove liquidity via Vault.exitPool() |

## Triggers
Activate this skill when users want to interact with Balancer V2 pools, including swapping tokens, providing/removing liquidity, or querying pool data on Arbitrum or Ethereum. Use for phrases like "swap on Balancer," "add liquidity to Balancer pool," or "show Balancer pools."
