
# raydium-plugin -- Skill Summary

## Overview
This plugin provides comprehensive access to Raydium's automated market maker (AMM) on Solana mainnet, enabling token swaps, price discovery, and pool analytics. It combines read-only operations for market data with secure write operations for executing trades, featuring built-in safety mechanisms including balance checks, price impact warnings, and transaction previews.

## Usage
Install the plugin and run commands like `raydium get-swap-quote` for quotes or `raydium swap` for trades. Always preview swaps with `--dry-run` and confirm with users before executing live transactions.

## Commands
| Command | Description |
|---------|-------------|
| `get-swap-quote` | Get expected output amount and price impact for a swap |
| `get-price` | Calculate price ratio between two tokens |
| `get-token-price` | Retrieve USD prices for token mint addresses |
| `get-pools` | Query pool information by mint addresses or pool IDs |
| `get-pool-list` | Browse paginated list of all Raydium pools |
| `swap` | Execute token swap (requires user confirmation) |

## Triggers
Activate when users mention Raydium operations like "swap on raydium", "raydium price", "raydium pool", or request token swaps, price quotes, or DEX operations on Solana. Always confirm before executing any swap transactions.
