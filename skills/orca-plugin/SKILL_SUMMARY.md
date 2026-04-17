
# orca-plugin -- Skill Summary

## Overview
This plugin provides access to Orca's concentrated liquidity AMM on Solana, enabling users to swap tokens, query pool information, and get swap quotes. It integrates with Orca's Whirlpools program to offer concentrated liquidity features with automatic safety checks including balance verification, security token scanning, and price impact protection.

## Usage
Start with `orca-plugin quickstart` to check your wallet status and get personalized next steps. Use `get-quote` to preview swap rates, then `swap` (preview mode) followed by `orca-plugin --confirm swap` to execute trades.

## Commands
| Command | Description |
|---------|-------------|
| `quickstart` | Check wallet status and get personalized onboarding steps |
| `get-pools` | List Whirlpool pools for a token pair with TVL data |
| `get-quote` | Get swap quote with estimated output and price impact |
| `swap` | Execute token swap (preview without --confirm, execute with --confirm) |

## Triggers
Activate when users want to swap tokens on Solana, query Orca pools, get swap quotes, or need help with Orca DEX operations. Also trigger for phrases like "orca swap", "whirlpool swap", or "swap tokens on solana".
