
# across -- Skill Summary

## Overview
The across skill enables cross-chain token bridging via the Across Protocol, supporting seamless transfers between Ethereum, Arbitrum, Base, Optimism, and Polygon networks. It provides comprehensive quote fetching, route discovery, transaction execution with automatic approval handling, and real-time status tracking for bridge operations involving USDC, WETH, ETH, and other ERC-20 tokens.

## Usage
Use `across get-quote` to fetch bridge quotes, `across get-routes` to discover available paths, and `across bridge` to execute cross-chain transfers. All bridge transactions require explicit user confirmation before execution.

## Commands
| Command | Description |
|---------|-------------|
| `get-quote` | Fetch cross-chain bridge quote with fees and timing |
| `get-routes` | List all available cross-chain routes with optional filtering |
| `get-limits` | Get transfer limits and liquidity info for specific routes |
| `bridge` | Execute cross-chain token bridge with approval handling |
| `get-status` | Check fill status of bridge deposits by transaction hash |

## Triggers
Activate this skill when users want to transfer tokens between different blockchain networks, need cross-chain bridge quotes or route information, or want to check the status of pending bridge transactions.
