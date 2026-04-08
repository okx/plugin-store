
# clanker -- Skill Summary

## Overview
This skill enables deployment and management of Clanker ERC-20 tokens on Base and Arbitrum networks. It provides comprehensive token lifecycle management including deployment with custom parameters, creator-based token search, real-time token information retrieval, and LP fee reward claiming for token creators.

## Usage
Use trigger phrases like "deploy token on Clanker", "search tokens by creator", or "claim LP rewards" to activate relevant commands. Token deployment requires a Clanker partner API key and user confirmation before execution.

## Commands
| Command | Purpose | Type |
|---------|---------|------|
| `list-tokens` | List recently deployed tokens with pagination | Read |
| `search-tokens --query <address\|username>` | Search tokens by creator address or Farcaster username | Read |
| `token-info --address <addr>` | Get token metadata and current price | Read |
| `deploy-token --name X --symbol Y --api-key K` | Deploy new ERC-20 token via Clanker | Write |
| `claim-rewards --token-address <addr>` | Claim LP fee rewards for token creators | Write |

## Triggers
Activate this skill when users want to launch meme tokens, search for tokens by specific creators, check token information, or claim creator rewards from Clanker-deployed tokens. Use for Clanker protocol interactions on Base and Arbitrum networks.
