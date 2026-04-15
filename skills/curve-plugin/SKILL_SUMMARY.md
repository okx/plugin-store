
# curve-plugin -- Skill Summary

## Overview
The Curve plugin enables AI agents to interact with Curve Finance, a leading decentralized exchange specialized in stablecoin trading and liquidity provision. It provides comprehensive functionality for swapping tokens, managing liquidity positions, and querying pool data across Ethereum, Arbitrum, Base, Polygon, and BSC networks. The plugin handles all the complexity of interacting with Curve's various pool types and registries while providing a simple command-line interface.

## Usage
Use trigger phrases like "swap on Curve", "add liquidity Curve", or "Curve pool APY" to activate the plugin. All write operations require user confirmation and support dry-run previews for safety.

## Commands
| Command | Description |
|---------|-------------|
| `get-pools` | List available Curve pools with TVL and APY data |
| `get-pool-info` | Get detailed information about a specific pool |
| `get-balances` | Check LP token balances across all pools |
| `quote` | Get swap quotes with slippage calculations |
| `swap` | Execute token swaps on Curve |
| `add-liquidity` | Add liquidity to Curve pools |
| `remove-liquidity` | Remove liquidity from pools (proportional or single-coin) |

## Triggers
Activate this skill when users want to trade stablecoins on Curve Finance, manage liquidity positions, or query Curve pool data. It should be used specifically for Curve-related DeFi operations across supported chains.
