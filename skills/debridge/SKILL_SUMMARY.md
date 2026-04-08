
# debridge -- Skill Summary

## Overview
The deBridge plugin enables cross-chain token bridging across multiple blockchain networks including Ethereum, Arbitrum, Base, Optimism, BSC, Polygon, and Solana through the Decentralized Liquidity Network (DLN). It provides quote fetching, transaction execution with automatic allowance handling, and order status tracking for seamless cross-chain asset transfers.

## Usage
Install the plugin via OKX plugin store, ensure your wallet is connected with `onchainos wallet login`, then use commands to get quotes, execute bridges, or check order status. The plugin handles ERC-20 approvals automatically and converts between different transaction formats for EVM and Solana chains.

## Commands
| Command | Description |
|---------|-------------|
| `debridge get-quote` | Fetch cross-chain swap quote without executing transaction |
| `debridge bridge` | Execute full cross-chain bridge with user confirmation |
| `debridge get-status` | Query order status by order ID |
| `debridge get-chains` | List all supported chains and their IDs |

## Triggers
Activate when users want to bridge tokens across different blockchain networks, especially when mentioning "cross-chain", "bridge", "deBridge", "move tokens", or specific chain-to-chain transfers like "Arbitrum to Base" or "Solana to EVM".
