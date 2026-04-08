
# debridge -- Skill Summary

## Overview
The deBridge plugin enables cross-chain token bridging through the deBridge Decentralized Liquidity Network (DLN), supporting seamless transfers between EVM chains (Ethereum, Arbitrum, Base, Optimism, BSC, Polygon, Avalanche) and Solana. It provides quote generation, transaction execution with proper confirmations, and order tracking capabilities for secure cross-chain asset transfers.

## Usage
Install via OKX plugin store, ensure wallet is connected with `onchainos wallet login`, then use commands like `debridge get-quote` to preview transfers or `debridge bridge` to execute cross-chain swaps. All write operations require user confirmation before execution.

## Commands
| Command | Description |
|---------|-------------|
| `get-quote` | Fetch cross-chain swap quote without creating transaction |
| `bridge` | Execute full cross-chain bridge with confirmations |
| `get-status` | Query order status by order ID |
| `get-chains` | List all supported chains and their IDs |

## Triggers
Activate when users need to bridge tokens across different blockchain networks, especially between EVM chains and Solana, or when they mention cross-chain swaps, deBridge, DLN, or moving tokens between supported networks.
