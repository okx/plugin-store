## Overview

Printr is a cross-chain token launchpad built for holders. The plugin lets an AI agent deploy, fund, stake, and graduate tokens on EVM chains and Solana through the Printr MCP server. Each token launches on a bonding curve and graduates to DEX liquidity once it crosses the configured threshold; holders earn through staking and creator-fee distribution.

Core operations:

- Launch a token on Base, Ethereum, Arbitrum, BSC, Avalanche, Solana, and other supported chains (one call or two-step build + sign)
- Quote launch costs (initial buy, deployment fee, gas) before committing
- Manage encrypted local wallets (`printr_wallet_*`) or use a separate treasury wallet to protect funds
- Stake on launched tokens, claim staking rewards, collect creator fees
- Transfer native and ERC-20 / SPL tokens, check balances
- Sign transactions via browser wallet (MetaMask / Phantom), an encrypted keystore, or an env-provided key for autonomous mode

Tags: `token-launch` `launchpad` `bonding-curve` `staking` `solana` `evm` `cross-chain`

## Prerequisites

- No IP/region restrictions imposed by the plugin; users must comply with Printr's own [terms](https://printr.money/terms)
- Supported chains: Base, Ethereum, Arbitrum, BSC, Avalanche, Unichain, Monad, Hyperliquid, Mantle, MegaETH, Solana (call `printr_supported_chains` for the live list)
- Supported tokens: native gas tokens for each chain plus any ERC-20 / SPL the user transfers; tokens launched by Printr are tracked by the plugin
- Required environment: a JavaScript runtime that can run the MCP server (`npx -y @printr/mcp@latest` or `bunx @printr/mcp@latest`)
- Optional credentials, all read from environment variables:
  - `PRINTR_API_KEY` — partner key (falls back to a documented public default)
  - `OPENROUTER_API_KEY` — enables `printr_generate_image` and auto image generation
  - `EVM_WALLET_PRIVATE_KEY` / `SVM_WALLET_PRIVATE_KEY` — autonomous signing
  - `PRINTR_DEPLOYMENT_PASSWORD` — encrypts the deployment wallet used by the treasury-protected flow
- A funded wallet on each chain you intend to launch from (or the browser-signer flow via `printr_open_web_signer`)

## Quick Start

1. **Add the Printr MCP server to your agent config**

   ```json
   {
     "mcpServers": {
       "printr": {
         "command": "npx",
         "args": ["-y", "@printr/mcp@latest"]
       }
     }
   }
   ```

   The default `PRINTR_API_KEY` is a public AI-integration key; replace it with a partner key for production usage.

2. **Quote a launch**

   Ask the agent: "Quote a launch on Base with $10 initial buy." The agent calls `printr_quote` and returns itemized costs (initial buy, deployment fee, gas) before any signing happens.

3. **Launch in one call (interactive)**

   Ask: "Launch a token named Moon Cat ($MCAT) on Solana with $10 initial buy." The agent calls `printr_launch_token` and either opens a browser signing session (`printr_open_web_signer`) or signs with a configured wallet.

4. **Launch in two steps (manual confirmation)**

   Ask the agent to build the payload first via `printr_create_token`, review the unsigned transaction, then sign with `printr_sign_and_submit_evm` or `printr_sign_and_submit_svm`.

5. **Manage funds**

   - `printr_set_treasury_wallet` + `printr_fund_deployment_wallet` create an ephemeral deployment wallet so your treasury is never exposed during launches
   - `printr_drain_deployment_wallet` returns unused funds after the launch
   - `printr_get_balance` / `printr_get_token_balance` report current balances
   - `printr_transfer` / `printr_transfer_token` move funds between addresses

6. **Trade, stake, and collect**

   - `printr_create_stake_position`, `printr_get_staking_positions`, `printr_claim_staking_rewards` manage stake positions on Printr tokens
   - `printr_get_creator_fees`, `printr_claim_fees` collect creator fees to your treasury

All chains are addressed via CAIP-2 (`eip155:8453`, `solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp`) and all accounts via CAIP-10. Run `printr_supported_chains` for the canonical list.
