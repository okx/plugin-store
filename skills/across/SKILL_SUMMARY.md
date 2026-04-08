
# across -- Skill Summary

## Overview
The across skill enables cross-chain token bridging via Across Protocol between Ethereum, Arbitrum, Base, Optimism, and Polygon. It provides comprehensive bridging functionality including quote generation, route discovery, transfer limits checking, and transaction execution with built-in safety measures requiring explicit user confirmation before any on-chain operations.

## Usage
Use `across get-quote` to fetch bridge quotes, `across get-routes` to discover available paths, and `across bridge` to execute cross-chain transfers. All bridge operations require explicit user confirmation and support dry-run mode for safe testing.

## Commands
| Command | Description |
|---------|-------------|
| `get-quote` | Fetch cross-chain bridge quote with fees and timing |
| `get-routes` | List available bridging routes between chains |
| `get-limits` | Get transfer limits and liquidity for specific routes |
| `bridge` | Execute cross-chain token bridge (requires user confirmation) |
| `get-status` | Check fill status of bridge deposits |

## Triggers
Activate this skill when users want to move tokens between different blockchain networks, need cross-chain liquidity, or require quotes for cross-chain transfers. Use for bridging operations specifically on Ethereum, Arbitrum, Base, Optimism, and Polygon networks.
