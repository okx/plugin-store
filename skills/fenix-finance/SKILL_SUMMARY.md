
# fenix-finance -- Skill Summary

## Overview
This skill provides comprehensive interaction with Fenix Finance V3 DEX on Blast network, enabling users to perform token swaps, manage concentrated liquidity positions, and query market data. Built on Algebra Integral V1 protocol, it offers dynamic fee pools without traditional fee tiers, supporting operations like adding/removing liquidity, collecting fees, and real-time price discovery through QuoterV2 integration.

## Usage
Use commands like `fenix-finance swap --token-in WETH --token-out USDB --amount 1` for swapping, or `fenix-finance add-liquidity --token0 USDB --token1 WETH --amount0 100 --amount1 0.05` for providing liquidity. All write operations include dry-run mode and require user confirmation before execution.

## Commands
| Command | Description |
|---------|-------------|
| `price` | Get swap quotes between token pairs |
| `swap` | Execute token swaps with slippage protection |
| `positions` | List all LP NFT positions for a wallet |
| `add-liquidity` | Mint new concentrated liquidity positions |
| `remove-liquidity` | Remove liquidity and collect fees from positions |
| `balance` | Show wallet token balances on Blast |

## Triggers
Activate when users mention Fenix Finance, Fenix DEX, swapping on Blast, concentrated liquidity on Blast, or need to interact with FNX tokens and Algebra AMM functionality. Do not use for other Blast DEXes like Thruster Finance or general Uniswap operations.
