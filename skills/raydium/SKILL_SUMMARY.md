
# raydium -- Skill Summary

## Overview
The Raydium plugin provides comprehensive access to the Raydium AMM on Solana, enabling users to swap tokens, query prices, and explore liquidity pools. It offers both read-only operations for market data and write operations for executing trades, with built-in safety mechanisms to protect against high-impact trades.

## Usage
Use commands like `raydium swap` to execute token swaps, `raydium get-price` for price queries, or `raydium get-pools` to explore liquidity pools. All swap operations require user confirmation and include dry-run capabilities for safe preview.

## Commands
| Command | Description |
|---------|-------------|
| `get-swap-quote` | Get expected output amount, price impact, and route for a swap |
| `get-price` | Get price ratio between two tokens |
| `get-token-price` | Get USD prices for one or more tokens |
| `get-pools` | Query pool information by mint addresses or pool IDs |
| `get-pool-list` | List all pools with pagination and sorting |
| `swap` | Execute token swap (requires user confirmation) |

## Triggers
Activate when users want to swap tokens on Raydium, check token prices, or explore liquidity pools on Solana. Also triggered by phrases like "raydium swap", "raydium price", or "raydium pool".
