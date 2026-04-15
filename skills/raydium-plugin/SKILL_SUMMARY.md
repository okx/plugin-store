
# raydium-plugin -- Skill Summary

## Overview
This plugin provides comprehensive access to Raydium's automated market maker (AMM) on Solana, enabling token swaps, price discovery, and liquidity pool analysis. It handles both read operations (quotes, prices, pool data) through direct API calls and write operations (swaps) via secure transaction building and submission to Solana mainnet.

## Usage
Install the plugin and use commands like `raydium swap` to execute trades or `raydium get-swap-quote` for price quotes. All swap operations require explicit user confirmation and support dry-run previews for safety.

## Commands
| Command | Description |
|---------|-------------|
| `get-swap-quote` | Get expected output amount and price impact for a potential swap |
| `get-price` | Calculate price ratio between two tokens |
| `get-token-price` | Fetch USD prices for one or more tokens |
| `get-pools` | Query pool information by token pairs or pool IDs |
| `get-pool-list` | Browse paginated list of all Raydium pools |
| `swap` | Execute token swap (requires user confirmation) |

## Triggers
Activate this skill when users mention Raydium trading, token swaps on Solana, getting DEX prices, or querying liquidity pool information. Also trigger on phrases like "swap on raydium", "raydium price", or "raydium pool".
