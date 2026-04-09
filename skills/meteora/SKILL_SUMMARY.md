
# meteora -- Skill Summary

## Overview
This skill provides comprehensive access to Meteora's Dynamic Liquidity Market Maker (DLMM) protocol on Solana. It enables users to search and analyze liquidity pools, obtain swap quotes, monitor LP positions, and execute token swaps through an integrated workflow that combines direct API calls for data retrieval with onchainos CLI for transaction execution.

## Usage
Install the meteora binary and use commands like `meteora get-pools` to search pools, `meteora get-swap-quote` for pricing, and `meteora swap` to execute trades. All swap operations require user confirmation and support dry-run mode for preview.

## Commands
| Command | Description |
|---------|-------------|
| `meteora get-pools` | Search and list DLMM pools with filtering and sorting options |
| `meteora get-pool-detail --address <pool>` | Get detailed information for a specific pool |
| `meteora get-swap-quote --from-token <mint> --to-token <mint> --amount <amount>` | Get swap quote estimation |
| `meteora get-user-positions [--wallet <address>] [--pool <pool>]` | View user's LP positions and unclaimed fees |
| `meteora swap --from-token <mint> --to-token <mint> --amount <amount> [--dry-run]` | Execute token swap with confirmation workflow |

## Triggers
Activate this skill when users want to interact with Meteora DLMM protocol, including phrases like "Meteora swap", "find Meteora pool", "check my Meteora positions", or "swap on Meteora". Also responds to Chinese equivalents like "Meteora换币" and "查询Meteora流动池".
