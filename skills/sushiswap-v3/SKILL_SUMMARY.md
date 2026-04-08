
# sushiswap-v3 -- Skill Summary

## Overview
This skill enables comprehensive interaction with SushiSwap V3's concentrated liquidity market maker protocol across 38+ EVM chains. It provides token swapping with automatic best-route discovery, full LP position management including minting NFT positions with custom tick ranges, fee collection, liquidity adjustments, and position burning. The skill handles both read-only operations via direct RPC calls and write operations through the onchainos wallet system with built-in approval management.

## Usage
Install the binary and ensure onchainos CLI is configured with a wallet. Use commands like `sushiswap-v3 quote` for gas-free price discovery and `sushiswap-v3 swap` for executing trades with automatic slippage protection.

## Commands
| Command | Description |
|---------|-------------|
| `quote` | Get swap quotes across all fee tiers (gas-free) |
| `swap` | Execute token swaps with slippage protection |
| `get-pools` | Query pool addresses by token pair |
| `get-positions` | List all LP positions owned by wallet |
| `add-liquidity` | Mint new concentrated liquidity positions |
| `remove-liquidity` | Decrease or remove liquidity from positions |
| `collect-fees` | Collect accumulated trading fees |

## Triggers
Activate this skill when users need to swap tokens on SushiSwap V3, provide concentrated liquidity with custom price ranges, manage existing LP positions, or collect trading fees across supported EVM chains. Use for DeFi operations requiring precise liquidity management or multi-chain token trading.
