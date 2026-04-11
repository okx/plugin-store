
# curve -- Skill Summary

## Overview
The Curve plugin enables interaction with Curve Finance, a decentralized exchange specialized in stablecoin trading and liquidity provision. It supports swapping stablecoins, managing liquidity positions, and querying pool data across Ethereum, Arbitrum, Base, Polygon, and BSC networks. The plugin handles token approvals automatically and provides comprehensive pool information including APY rates and TVL data.

## Usage
Use trigger phrases like "swap on Curve", "add liquidity Curve", or "Curve pool APY" to activate the plugin. All write operations require user confirmation and support dry-run previews before execution.

## Commands
| Command | Description |
|---------|-------------|
| `get-pools` | List available Curve pools with TVL and APY data |
| `get-pool-info` | Get detailed information about a specific pool |
| `get-balances` | Check LP token balances for a wallet |
| `quote` | Get swap quotes between tokens |
| `swap` | Execute token swaps on Curve |
| `add-liquidity` | Add liquidity to Curve pools |
| `remove-liquidity` | Remove liquidity from pools (proportional or single-coin) |

## Triggers
Activate when users want to perform stablecoin swaps on Curve, manage liquidity positions, or query Curve pool data including APY rates and balances. Use for Curve-specific operations rather than general DEX activities on other protocols.
