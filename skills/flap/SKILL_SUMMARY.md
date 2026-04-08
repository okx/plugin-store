
# flap -- Skill Summary

## Overview
The flap skill enables interaction with Flap Protocol bonding curves on BSC, allowing users to create new tokens (standard or tax variants), buy tokens with BNB, sell tokens for BNB, and query bonding curve state. It supports both simple token launches and advanced tax token configurations with customizable rates and beneficiaries, while providing safety features like sell tax warnings and automatic DEX graduation detection.

## Usage
Run `flap --version` to verify installation, ensure your wallet is connected with `onchainos wallet status`, then use commands with `--dry-run` first to preview operations before executing on-chain transactions.

## Commands
| Command | Description |
|---------|-------------|
| `get-token-info --token <address>` | Query bonding curve state, price, and reserves |
| `create-token --name <name> --symbol <symbol>` | Launch new token on bonding curve |
| `buy --token <address> --bnb-amount <wei>` | Buy tokens with BNB |
| `sell --token <address> --token-amount <units>` | Sell tokens for BNB |

All write commands support `--dry-run` for previewing and require explicit user confirmation.

## Triggers
Activate this skill when users want to create tokens on Flap, buy/sell Flap tokens, check bonding curves, launch meme tokens on BSC, or interact with flap.sh launchpad functionality. Do not use for established ERC-20 tokens or non-Flap Protocol tokens.
