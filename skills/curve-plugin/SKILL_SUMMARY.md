
# curve -- Skill Summary

## Overview
The Curve plugin enables interaction with Curve Finance, a decentralized exchange optimized for stablecoin trading with low slippage. It provides comprehensive functionality for swapping tokens, managing liquidity positions, and querying pool data across five major blockchain networks including Ethereum, Arbitrum, Base, Polygon, and BSC.

## Usage
Trigger the plugin with phrases like "swap on Curve", "Curve pool APY", "add liquidity Curve", or "Curve quote". All write operations use dry-run previews and require user confirmation before execution.

## Commands
| Command | Purpose |
|---------|---------|
| `get-pools` | List available Curve pools with TVL and APY data |
| `get-pool-info` | Get detailed information about a specific pool |
| `get-balances` | Check LP token balances for a wallet |
| `quote` | Get swap quotes between tokens |
| `swap` | Execute token swaps on Curve |
| `add-liquidity` | Add liquidity to Curve pools |
| `remove-liquidity` | Remove liquidity from pools (proportional or single-coin) |

## Triggers
Activate this skill when users want to perform stablecoin swaps, manage liquidity on Curve Finance, or query Curve pool information and APY rates. Use for phrases containing "Curve swap", "Curve liquidity", "Curve pool", or "Curve quote".
