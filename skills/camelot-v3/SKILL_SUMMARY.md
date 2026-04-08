
# camelot-v3 -- Skill Summary

## Overview
This skill provides comprehensive access to Camelot V3, Arbitrum's native concentrated liquidity DEX built on the Algebra V1 protocol. It enables users to get price quotes, execute token swaps, and manage liquidity positions on Arbitrum without requiring fee tier selection since each token pair has a single pool.

## Usage
Install the plugin and ensure onchainos CLI is available, then use commands like `camelot-v3 quote` for price checks or `camelot-v3 swap --confirm` for executing trades. All write operations require explicit confirmation before broadcasting transactions.

## Commands
| Command | Purpose |
|---------|---------|
| `quote` | Get price quotes for token swaps (read-only) |
| `swap` | Execute token swaps on Camelot V3 |
| `positions` | List your concentrated liquidity positions |
| `add-liquidity` | Add liquidity to pools and earn fees |
| `remove-liquidity` | Remove liquidity from existing positions |

## Triggers
Activate this skill when users want to trade tokens, check prices, or manage liquidity on Arbitrum using Camelot V3, especially when they mention concentrated liquidity, LP positions, or Arbitrum-native DEX operations.
