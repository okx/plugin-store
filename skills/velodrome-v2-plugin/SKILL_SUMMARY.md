
# velodrome-v2-plugin -- Skill Summary

## Overview
This skill enables interaction with Velodrome V2, the largest DEX on Optimism, providing comprehensive AMM functionality for both volatile and stable pools. Users can swap tokens, manage liquidity positions, and claim VELO rewards through the classic constant-product and stable-curve AMM models.

## Usage
Install the plugin and use commands like `velodrome-v2 swap --token-in WETH --token-out USDC --amount-in 0.00005` for swapping or `velodrome-v2 add-liquidity --token-a WETH --token-b USDC --amount-a-desired 0.00005` for providing liquidity. All write operations require `--confirm` flag after reviewing transaction details.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get swap quote without transaction |
| `swap` | Execute token swaps via Router |
| `pools` | Query pool info and reserves |
| `positions` | View LP token balances |
| `add-liquidity` | Add liquidity to pools |
| `remove-liquidity` | Remove LP tokens |
| `claim-rewards` | Claim VELO gauge emissions |

## Triggers
Activate this skill when users want to trade tokens, provide liquidity, or manage positions on Velodrome V2 on Optimism. Use for DeFi activities involving WETH, USDC, VELO, and other Optimism tokens through AMM pools.
