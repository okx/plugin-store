
# balancer-v2 -- Skill Summary

## Overview
The balancer-v2 skill provides comprehensive access to Balancer V2, a multi-token automated market maker (AMM) DEX on Ethereum and Arbitrum. It enables users to swap tokens, query pool information, obtain accurate swap quotes, manage liquidity positions, and track LP holdings through Balancer's unified Vault architecture.

## Usage
Install the plugin and ensure onchainos CLI is available, then use commands like `balancer-v2 pools` to discover pools or `balancer-v2 swap` to execute trades. All write operations require the `--confirm` flag and explicit user approval before broadcasting transactions.

## Commands
| Command | Description |
|---------|-------------|
| `pools` | List top Balancer V2 pools by liquidity |
| `pool-info` | Get detailed pool information (tokens, balances, weights) |
| `quote` | Get swap quotes via on-chain BalancerQueries contract |
| `positions` | View LP positions and BPT holdings for connected wallet |
| `swap` | Execute token swaps through Vault.swap() |
| `join` | Add liquidity to pools via Vault.joinPool() |
| `exit` | Remove liquidity from pools via Vault.exitPool() |

## Triggers
Activate this skill when users want to interact with Balancer V2 DEX, including swapping tokens, checking pool information, managing liquidity positions, or getting swap quotes on Arbitrum or Ethereum networks. The skill should also trigger when users mention Balancer-specific terms like BPT, weighted pools, or the Vault contract.
