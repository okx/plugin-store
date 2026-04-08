
# curve -- Skill Summary

## Overview
The curve plugin enables interaction with Curve Finance, a specialized DEX for stablecoin trading and liquidity provision. It supports swapping stablecoins with minimal slippage, adding/removing liquidity from pools, querying pool data and APY rates, and managing LP positions across five major chains (Ethereum, Arbitrum, Base, Polygon, BSC). All write operations include safety checks and require user confirmation.

## Usage
Install the plugin via the OKX plugin store, ensure your wallet is connected with `onchainos wallet login`, then use commands like `curve get-pools` to browse pools or `curve swap` to execute trades. Always preview transactions with `--dry-run` before confirming.

## Commands
| Command | Description |
|---------|-------------|
| `get-pools` | List Curve pools with TVL and APY data |
| `get-pool-info` | Get detailed information about a specific pool |
| `get-balances` | Check LP token balances in wallet |
| `quote` | Get swap quote with price impact and slippage |
| `swap` | Execute token swap on Curve |
| `add-liquidity` | Add liquidity to a Curve pool |
| `remove-liquidity` | Remove liquidity from a Curve pool |

## Triggers
Activate this skill when users want to swap stablecoins, manage Curve liquidity positions, or query Curve pool data. Trigger phrases include "swap on Curve", "Curve pool APY", "add liquidity Curve", "remove liquidity Curve", and "Curve pools".
