
# camelot-v3 -- Skill Summary

## Overview
Camelot V3 is a concentrated liquidity DEX plugin for Arbitrum that enables token swaps, price quotes, and liquidity position management. Built on the Algebra V1 protocol, it provides a native Arbitrum trading experience with single pools per token pair and concentrated liquidity features similar to Uniswap V3.

## Usage
Use the `camelot-v3` command with subcommands like `quote`, `swap`, `positions`, `add-liquidity`, and `remove-liquidity`. All operations target Arbitrum (chain ID 42161) and write operations require user confirmation before broadcasting.

## Commands
| Command | Description |
|---------|-------------|
| `camelot-v3 quote` | Get price quotes for token swaps (read-only) |
| `camelot-v3 swap` | Execute token swaps on Camelot V3 |
| `camelot-v3 positions` | List your LP positions |
| `camelot-v3 add-liquidity` | Add concentrated liquidity to pools |
| `camelot-v3 remove-liquidity` | Remove liquidity from positions |

## Triggers
Activate this skill when users want to trade tokens, check prices, or manage liquidity positions on Camelot V3 DEX on Arbitrum. Use for concentrated liquidity operations and when users specifically mention Camelot, Arbitrum DEX trading, or Algebra protocol interactions.
