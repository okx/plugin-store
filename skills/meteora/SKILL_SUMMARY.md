
# meteora -- Skill Summary

## Overview
The meteora skill provides comprehensive access to Meteora's Dynamic Liquidity Market Maker (DLMM) protocol on Solana, enabling users to search liquidity pools, obtain swap quotes, manage LP positions, execute token swaps, and provide/remove liquidity. It integrates with the onchainos CLI for wallet operations and transaction execution, supporting both read-only operations via REST API calls and write operations that require user confirmation and wallet signing.

## Usage
Install the meteora binary and use commands like `meteora get-pools` to search pools, `meteora swap` to execute token swaps, or `meteora add-liquidity` to provide liquidity. All write operations support `--dry-run` mode for preview before execution.

## Commands
| Command | Description |
|---------|-------------|
| `meteora get-pools` | Search and list DLMM pools with filtering and sorting options |
| `meteora get-pool-detail --address <pool>` | Get detailed information for a specific pool |
| `meteora get-swap-quote --from-token <mint> --to-token <mint> --amount <amount>` | Get swap quote estimation |
| `meteora get-user-positions [--wallet <addr>] [--pool <addr>]` | View user's LP positions |
| `meteora swap --from-token <mint> --to-token <mint> --amount <amount> [--dry-run]` | Execute token swap with preview option |
| `meteora add-liquidity --pool <addr> --amount-x <amt> --amount-y <amt> [--dry-run]` | Add liquidity to DLMM pool |
| `meteora remove-liquidity --pool <addr> --position <addr> [--pct <1-100>] [--close] [--dry-run]` | Remove liquidity from position |

## Triggers
Activate when users mention Meteora swaps, DLMM pools, liquidity provision, or position management on Solana, including phrases like "swap on Meteora", "find Meteora pool", "add liquidity to Meteora", or "check my Meteora positions".
