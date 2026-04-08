
# pump-fun -- Skill Summary

## Overview
This skill enables interaction with pump.fun bonding curves on Solana mainnet, providing comprehensive token operations including buying, selling, creating new tokens, and monitoring bonding curve states. It handles both active bonding curves and graduated tokens that have moved to regular DEX trading, with built-in safety features like dry-run previews and user confirmation requirements for all write operations.

## Usage
Use natural language triggers like "buy pump.fun token", "sell pump.fun token", "create token pump.fun", or "check pump.fun price" to activate operations. All write operations require explicit user confirmation after showing a dry-run preview.

## Commands
| Command | Description |
|---------|-------------|
| `pump-fun get-token-info --mint <ADDRESS>` | Fetch bonding curve state and graduation progress |
| `pump-fun get-price --mint <ADDRESS> --direction <buy/sell> --amount <AMOUNT>` | Calculate buy/sell price for given amount |
| `pump-fun buy --mint <ADDRESS> --sol-amount <LAMPORTS> [--dry-run]` | Buy tokens on bonding curve |
| `pump-fun sell --mint <ADDRESS> [--token-amount <AMOUNT>] [--dry-run]` | Sell tokens back to bonding curve |
| `pump-fun create-token --name <NAME> --symbol <SYMBOL> --description <DESC> --image-path <PATH> [--dry-run]` | Deploy new token with bonding curve |

## Triggers
Activate this skill when users want to interact with pump.fun tokens, memecoins, or bonding curve mechanics on Solana. Trigger on phrases about buying/selling pump.fun tokens, creating new tokens, checking prices, or monitoring bonding curve graduation status.
