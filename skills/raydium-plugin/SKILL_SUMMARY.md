
# raydium-plugin -- Skill Summary

## Overview
This plugin provides comprehensive access to Raydium AMM functionality on Solana, enabling AI agents to execute token swaps, retrieve price data, and query liquidity pool information. It includes safety features like balance validation, price impact warnings, and dry-run capabilities to protect users from unfavorable trades.

## Usage
Install via the auto-injected setup commands, then use commands like `raydium get-swap-quote` for price discovery or `raydium swap` for executing trades. Always run swaps with `--dry-run` first and confirm with users before executing.

## Commands
| Command | Description |
|---------|-------------|
| `get-swap-quote` | Get expected output amount and price impact for a potential swap |
| `get-price` | Calculate price ratio between two tokens |
| `get-token-price` | Retrieve USD prices for specified token mints |
| `get-pools` | Query pool information by mint addresses or pool IDs |
| `get-pool-list` | List pools with pagination and sorting options |
| `swap` | Execute token swaps on Raydium (requires user confirmation) |

## Triggers
Activate when users mention Raydium-specific operations like "swap on raydium", "raydium price", "raydium pool", or general Solana DEX activities that would benefit from Raydium's liquidity and routing.
