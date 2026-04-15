
# curve-plugin -- Skill Summary

## Overview
The curve-plugin enables AI agents to interact with Curve Finance, a leading decentralized exchange specializing in stablecoin trading and liquidity provision. It provides comprehensive functionality for swapping tokens, managing liquidity positions, and querying pool data across Ethereum, Arbitrum, Base, Polygon, and BSC networks. The plugin emphasizes safety with confirmation gates for write operations and automatic handling of ERC-20 approvals.

## Usage
Install the plugin and run commands prefixed with `curve --chain <chain_id>`. All write operations require the `--confirm` flag to execute on-chain, otherwise they return a preview JSON for user confirmation.

## Commands
| Command | Description |
|---------|-------------|
| `get-pools` | List Curve pools with TVL and APY data |
| `get-pool-info --pool <address>` | Get detailed pool information including fees and virtual price |
| `get-balances` | Show LP token balances across all Curve positions |
| `quote --token-in <token> --token-out <token> --amount <amount>` | Get swap quote without execution |
| `swap --token-in <token> --token-out <token> --amount <amount> [--confirm]` | Execute token swap (requires --confirm) |
| `add-liquidity --pool <address> --amounts <amounts> [--confirm]` | Add liquidity to pool (requires --confirm) |
| `remove-liquidity --pool <address> [--coin-index <i>] [--confirm]` | Remove liquidity from pool (requires --confirm) |

## Triggers
Activate this skill when users mention Curve DEX operations like "swap on Curve", "Curve pool APY", "add liquidity to Curve", or "remove Curve LP tokens". Also trigger for stablecoin swaps where Curve offers better rates than other DEXs.
