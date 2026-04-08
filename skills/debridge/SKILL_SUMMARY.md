
# debridge -- Skill Summary

## Overview
The deBridge skill enables cross-chain token bridging between EVM chains (Ethereum, Arbitrum, Base, Optimism, BSC, Polygon) and Solana using the deBridge Decentralized Liquidity Network (DLN). It provides quote fetching, transaction execution with automatic approval handling, and order status tracking for secure cross-chain asset transfers.

## Usage
Use `debridge get-quote` to preview swap rates and fees, then `debridge bridge` to execute the cross-chain transfer after user confirmation. The skill handles ERC-20 approvals automatically and provides transaction status tracking via order IDs.

## Commands
| Command | Purpose |
|---------|---------|
| `get-quote` | Fetch cross-chain swap quote without executing transaction |
| `bridge` | Execute full cross-chain bridge with user confirmations |
| `get-status` | Query bridge order status by order ID |
| `get-chains` | List all supported chains and their IDs |

## Triggers
Activate when users want to bridge tokens between different blockchains, especially between EVM chains and Solana, or when they need cross-chain swap quotes and transaction status updates. Common trigger phrases include "bridge tokens", "cross-chain swap", "move tokens across chains", and "deBridge DLN".
