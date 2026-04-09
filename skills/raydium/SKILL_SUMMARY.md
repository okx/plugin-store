
# raydium -- Skill Summary

## Overview
This plugin provides comprehensive access to Raydium, a leading automated market maker (AMM) on Solana. It enables token swaps with safety protections, real-time price queries, and detailed liquidity pool information. The plugin integrates with the onchainos wallet system for secure transaction execution and includes safety features like price impact warnings and dry-run capabilities.

## Usage
Use trigger phrases like "swap on raydium", "raydium price", or "raydium pool" to activate the plugin. All swap operations require explicit user confirmation and include safety checks for price impact protection.

## Commands
| Command | Description |
|---------|-------------|
| `get-swap-quote` | Get expected output amount and price impact for a token swap |
| `get-price` | Calculate price ratio between two tokens |
| `get-token-price` | Get USD prices for token mint addresses |
| `get-pools` | Query pool information by tokens or pool IDs |
| `get-pool-list` | Browse paginated list of all Raydium pools |
| `swap` | Execute token swaps with confirmation and safety checks |

## Triggers
Activate when users want to swap tokens on Solana, check Raydium prices, or explore liquidity pools. Use when phrases mention "raydium", "swap solana", or requests for DEX operations on Solana mainnet.
