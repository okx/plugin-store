
# meteora -- Skill Summary

## Overview
The meteora plugin provides comprehensive access to Meteora's Dynamic Liquidity Market Maker (DLMM) on Solana, enabling users to search pools, obtain swap quotes, manage liquidity positions, execute trades, and add/remove liquidity. It combines read-only operations via REST API calls with transaction execution through the onchainos framework, offering both dry-run previews and confirmed operations for swaps and liquidity management.

## Usage
Install the meteora binary and use commands like `meteora get-pools` to search pools, `meteora swap` to execute trades, and `meteora add-liquidity` to provide liquidity. All transaction operations support `--dry-run` for previewing before execution.

## Commands
| Command | Description |
|---------|-------------|
| `meteora get-pools` | Search and list DLMM pools with filtering and sorting options |
| `meteora get-pool-detail --address <pool>` | Get detailed information for a specific pool |
| `meteora get-swap-quote --from-token <mint> --to-token <mint> --amount <amount>` | Get swap quote between tokens |
| `meteora get-user-positions [--wallet <address>] [--pool <pool>]` | View user's LP positions |
| `meteora swap --from-token <mint> --to-token <mint> --amount <amount> [--dry-run]` | Execute token swap with optional preview |
| `meteora add-liquidity --pool <pool> [--amount-x <x>] [--amount-y <y>] [--dry-run]` | Add liquidity to DLMM pool |
| `meteora remove-liquidity --pool <pool> --position <pos> [--pct <pct>] [--close] [--dry-run]` | Remove liquidity from position |

## Triggers
Activate this skill when users want to trade tokens on Solana, provide liquidity to earn fees, check their LP positions, or discover high-yield pools on Meteora DLMM. Use for both read-only operations like pool discovery and transactional operations like swapping and liquidity management.
