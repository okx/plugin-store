
# four-meme -- Skill Summary

## Overview
The four-meme plugin enables trading of meme tokens on the Four.meme bonding curve launchpad on BNB Chain (BSC). It provides functionality to buy tokens with BNB, sell tokens back to the bonding curve, retrieve token information including price and market cap, and list supported base tokens from the platform configuration.

## Usage
Use the plugin to trade meme tokens on Four.meme by specifying token addresses and amounts. Add the `--confirm` flag to broadcast transactions on-chain, otherwise commands show previews only.

## Commands
| Command | Description |
|---------|-------------|
| `four-meme tokens` | List supported base tokens and platform configuration |
| `four-meme info --token <address>` | Get token details, price, market cap, and bonding curve progress |
| `four-meme buy --token <address> --amount-bnb <amount> [--confirm]` | Buy meme tokens from bonding curve using BNB |
| `four-meme sell --token <address> --amount <amount> [--confirm]` | Sell meme tokens back to bonding curve for BNB |

## Triggers
An AI agent should activate this skill when users want to trade meme tokens on Four.meme, check meme token prices or market data, or interact with bonding curve launchpads on BNB Chain.
