
# clanker -- Skill Summary

## Overview
The clanker skill enables deployment and management of ERC-20 tokens through the Clanker protocol on Base and Arbitrum networks. It provides comprehensive token lifecycle management including deployment, discovery, monitoring, and reward claiming functionality with built-in security features and dry-run capabilities for safe operations.

## Usage
Install the plugin via OKX plugin store, connect your wallet with `onchainos wallet login`, then use natural language commands like "deploy token on Base" or "claim my Clanker rewards". For deployments, you'll need a Clanker partner API key.

## Commands
| Command | Purpose |
|---------|---------|
| `list-tokens` | List recently deployed tokens with pagination |
| `search-tokens --query <creator>` | Search tokens by creator address or username |
| `token-info --address <addr>` | Get token metadata and current price |
| `deploy-token --name X --symbol Y --api-key K` | Deploy new ERC-20 token via Clanker |
| `claim-rewards --token-address <addr>` | Claim LP fee rewards for owned tokens |

## Triggers
Activate when users want to deploy tokens ("launch token on Clanker", "create ERC-20 on Base"), search for existing tokens by creator, or claim rewards from their deployed tokens. Also triggers for general Clanker token discovery and monitoring requests.
