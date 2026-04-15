
# orca-plugin -- Skill Summary

## Overview
This plugin enables AI agents to interact with Orca's Whirlpools concentrated liquidity AMM on Solana. It provides secure token swapping capabilities with built-in safety checks, pool discovery, and real-time quote generation. The plugin integrates directly with Orca's API for read operations and uses onchainos for secure transaction execution.

## Usage
Install the plugin and use commands like `orca get-pools` to discover liquidity pools, `orca get-quote` for swap estimates, and `orca swap` to execute trades. Always run with `--dry-run` first and confirm with the user before executing real swaps.

## Commands
| Command | Description |
|---------|-------------|
| `orca get-pools` | Query Whirlpool pools for token pairs, sorted by TVL |
| `orca get-quote` | Calculate estimated swap output for given input amount |
| `orca swap` | Execute token swap with safety checks and user confirmation |

## Triggers
Activate this skill when users want to swap tokens on Solana, query Orca liquidity pools, get swap quotes, or interact with the Whirlpools DEX. Also responds to phrases like "orca swap", "swap on orca", "whirlpool swap", or "get swap quote".
