
# clanker -- Skill Summary

## Overview
The clanker skill provides comprehensive token management for the Clanker protocol, enabling users to deploy new ERC-20 tokens on Base and Arbitrum networks, search and discover existing tokens by creator, and claim liquidity provider fee rewards. It integrates with the OKX OnChainOS wallet system and includes built-in security scanning to protect users from malicious tokens.

## Usage
Install the plugin via OKX plugin store, connect your wallet with `onchainos wallet login`, then use trigger phrases like "deploy token on Clanker", "search tokens by creator", or "claim my LP rewards". Write operations require user confirmation before execution.

## Commands
| Command | Description | Type |
|---------|-------------|------|
| `list-tokens` | List recently deployed tokens with pagination | Read |
| `search-tokens` | Search tokens by creator address or username | Read |
| `token-info` | Get token details, price, and market cap | Read |
| `deploy-token` | Deploy new ERC-20 token (requires API key) | Write |
| `claim-rewards` | Claim LP fee rewards for token creators | Write |

## Triggers
Activate when users want to launch new tokens, discover existing Clanker tokens by creator, check token information, or claim creator rewards from LP fees. Also triggered by phrases mentioning Base/Arbitrum token deployment or Clanker protocol interactions.
