
# pump-fun-plugin -- Skill Summary

## Overview
This skill enables AI agents to interact with pump.fun bonding curves on Solana mainnet, providing capabilities to buy and sell memecoins, check token prices, and monitor bonding curve graduation progress. It integrates with onchainos for secure transaction execution and supports both tokens still on bonding curves and those that have graduated to DEX trading.

## Usage
Install the plugin via the auto-injected setup commands, then use CLI commands like `pump-fun get-token-info --mint <address>` to check token status or `pump-fun buy --mint <address> --sol-amount 0.01` to purchase tokens. Always run operations with `--dry-run` first and confirm with users before executing transactions.

## Commands
| Command | Description |
|---------|-------------|
| `pump-fun get-token-info --mint <address>` | Fetch bonding curve state, reserves, and graduation progress |
| `pump-fun get-price --mint <address> --direction <buy/sell> --amount <value>` | Calculate expected output for buy/sell operations |
| `pump-fun buy --mint <address> --sol-amount <value> [--dry-run]` | Buy tokens on bonding curve (preview with --dry-run) |
| `pump-fun sell --mint <address> [--token-amount <value>] [--dry-run]` | Sell tokens back to bonding curve (preview with --dry-run) |

## Triggers
Activate this skill when users mention buying or selling pump.fun tokens, checking pump.fun prices, or monitoring bonding curve progress. Also trigger for Chinese phrases like "购买pump.fun代币" or "查询pump.fun价格".
