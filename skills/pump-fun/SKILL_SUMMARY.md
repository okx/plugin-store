
# pump-fun -- Skill Summary

## Overview
This skill enables AI agents to interact with pump.fun bonding curves on Solana mainnet, providing capabilities to buy and sell tokens, check prices, and monitor bonding curve progress. It integrates with onchainos for secure transaction execution and provides real-time market data through direct Solana RPC calls.

## Usage
Install the pump-fun binary and use the provided commands to interact with tokens. Always run with --dry-run first for write operations and confirm with the user before executing transactions.

## Commands
| Command | Description |
|---------|-------------|
| `pump-fun get-token-info --mint <ADDRESS>` | Fetch bonding curve state and token information |
| `pump-fun get-price --mint <ADDRESS> --direction <buy/sell> --amount <AMOUNT>` | Calculate expected buy/sell price |
| `pump-fun buy --mint <ADDRESS> --sol-amount <AMOUNT> [--dry-run]` | Buy tokens on bonding curve |
| `pump-fun sell --mint <ADDRESS> [--token-amount <AMOUNT>] [--dry-run]` | Sell tokens back to bonding curve |

## Triggers
Activate this skill when users want to interact with pump.fun tokens, check token prices, monitor bonding curve progress, or execute trades on Solana memecoins. Also triggered by phrases like "buy pump.fun token", "sell pump.fun token", or "check pump.fun price".
