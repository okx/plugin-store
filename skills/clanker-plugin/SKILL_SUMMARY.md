
# clanker-plugin -- Skill Summary

## Overview
The clanker-plugin enables AI agents to interact with the Clanker token deployment platform on Base and Arbitrum networks. It provides comprehensive token lifecycle management including direct on-chain deployment via the Clanker V4 factory, creator-based token discovery, real-time price monitoring, and LP fee reward collection. The plugin integrates with onchainos for secure wallet operations and includes automated security scanning for token safety verification.

## Usage
Install via `npx skills add okx/plugin-store --skill clanker-plugin --yes --global` and ensure onchainos is logged in with `onchainos login`. Use trigger phrases like "deploy token on Clanker", "search tokens by creator", or "claim LP rewards" to activate specific functions.

## Commands
| Command | Description |
|---------|-------------|
| `clanker list-tokens` | List recently deployed Clanker tokens with pagination |
| `clanker search-tokens --query <address\|username>` | Search tokens by creator address or Farcaster username |
| `clanker token-info --address <addr>` | Get token metadata and price information |
| `clanker deploy-token --name X --symbol Y` | Deploy new ERC-20 token via Clanker (Base only) |
| `clanker claim-rewards --token-address <addr>` | Claim LP fee rewards for deployed tokens |

## Triggers
Activate when users want to deploy new meme tokens, search for tokens by specific creators, check token prices/metadata, or claim creator rewards from Clanker-deployed tokens. Best suited for Base and Arbitrum ecosystem token operations.
