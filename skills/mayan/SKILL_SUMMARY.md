
# mayan -- Skill Summary

## Overview
The Mayan plugin enables cross-chain token swaps between Solana and major EVM chains (Ethereum, Arbitrum, Base, Optimism, Polygon, BSC, Avalanche) using three routing protocols: Swift for speed (~15s), MCTP for stablecoin optimization, and Wormhole for reliability. It automatically handles transaction complexities including ERC-20 approvals, route selection, and cross-chain settlement tracking.

## Usage
Install via OKX plugin store, ensure wallet is connected with `onchainos wallet login`, then use `mayan get-quote` to preview rates and `mayan swap` to execute cross-chain transfers. The plugin automatically selects optimal routes and handles chain-specific transaction formatting.

## Commands
| Command | Description |
|---------|-------------|
| `mayan get-quote` | Fetch cross-chain swap quotes with rates, fees, and ETAs |
| `mayan swap` | Execute cross-chain token swap with automatic route selection |
| `mayan get-status` | Check swap progress using transaction hash |

## Triggers
Activate when users want to move tokens between different blockchains (Solana ↔ EVM chains), need cross-chain liquidity, or want to bridge assets for DeFi opportunities on other networks. Best for amounts requiring speed (Swift) or stablecoin transfers (MCTP).
