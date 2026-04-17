
# meteora-plugin -- Skill Summary

## Overview
The meteora-plugin provides comprehensive access to Meteora's Dynamic Liquidity Market Maker (DLMM) protocol on Solana, enabling users to search liquidity pools, get swap quotes, manage LP positions, execute swaps, and add/remove liquidity. The plugin integrates with the onchainos CLI for secure transaction signing and uses both REST API calls for read operations and on-chain queries for position data.

## Usage
Install the plugin using the auto-injected setup commands, then use `meteora-plugin <subcommand>` for operations. Write operations (swap, add-liquidity, remove-liquidity) require the `--confirm` global flag placed before the subcommand to execute transactions.

## Commands
| Command | Description |
|---------|-------------|
| `meteora-plugin get-pools` | Search and list DLMM pools with filtering and sorting |
| `meteora-plugin get-pool-detail --address <pool>` | Get detailed pool information |
| `meteora-plugin get-swap-quote --from-token <mint> --to-token <mint> --amount <amount>` | Get swap quote estimates |
| `meteora-plugin get-user-positions [--wallet <addr>] [--pool <pool>]` | View user LP positions |
| `meteora-plugin swap --from-token <mint> --to-token <mint> --amount <amount>` | Execute token swaps (add --confirm to execute) |
| `meteora-plugin add-liquidity --pool <addr> --amount-x <x> --amount-y <y>` | Add liquidity to pools (add --confirm to execute) |
| `meteora-plugin remove-liquidity --pool <addr> --position <pos> [--pct <n>] [--close]` | Remove liquidity (add --confirm to execute) |
| `meteora-plugin quickstart --pool <addr>` | Check balances and get recommended commands |

## Triggers
An AI agent should activate this skill when users want to interact with Meteora DLMM pools on Solana, including swapping tokens, providing liquidity, checking LP positions, or exploring available pools and yields. The quickstart command should be used for new users asking how to get started with Meteora.
