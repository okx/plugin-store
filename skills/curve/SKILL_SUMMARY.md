
# curve -- Skill Summary

## Overview
The curve plugin enables interaction with Curve Finance, a specialized DEX for stablecoin trading and liquidity provision. It supports swapping, liquidity management, pool queries, and APY tracking across Ethereum, Arbitrum, Base, Polygon, and BSC networks.

## Usage
Install the plugin via OKX plugin store, ensure onchainos wallet is connected, then use natural language commands like "swap USDC for DAI on Curve" or "add liquidity to Curve 3pool".

## Commands
| Command | Purpose |
|---------|---------|
| `curve get-pools` | List available Curve pools with TVL and APY |
| `curve get-pool-info` | Get detailed information about a specific pool |
| `curve get-balances` | Check LP token balances for connected wallet |
| `curve quote` | Get swap quotes with price impact and slippage |
| `curve swap` | Execute stablecoin swaps on Curve |
| `curve add-liquidity` | Add liquidity to Curve pools |
| `curve remove-liquidity` | Remove liquidity from Curve pools |

## Triggers
Activate this skill when users mention Curve-specific operations like "swap on Curve", "Curve pool APY", "add liquidity Curve", or when they need stablecoin trading with minimal slippage and fees.
