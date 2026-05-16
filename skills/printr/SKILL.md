---
name: printr
description: "Token launchpad built for holders. Deploy, fund, sign, stake, and graduate tokens across EVM chains and Solana via the Printr MCP server."
version: "0.16.0"
author: "PrintrFi"
tags:
  - token-launch
  - launchpad
  - solana
  - evm
  - cross-chain
  - mcp
---

# Printr

## Overview

This skill lets an AI agent launch and manage tokens on Base, Ethereum, Arbitrum, BSC, Avalanche, Solana, and other EVM and SVM chains through the [Printr MCP server](https://github.com/PrintrFi/printr-mcp). Printr's launchpad primitives are bonding-curve launches, holder staking, creator-fee distribution, and on-chain graduation to DEX liquidity once a token's curve crosses its threshold. The plugin orchestrates token creation, ephemeral deployment-wallet funding, signing (browser, encrypted keystore, or env-provided key), staking, fee claiming, and balance / transfer tooling.

Every chain is addressed by CAIP-2; every account by CAIP-10. Call `printr_supported_chains` for the canonical chain list at runtime, and `printr_get_deployments` to inspect a token's per-chain graduation status.

## Pre-flight Checks

Before invoking any command, ensure the MCP server is reachable. From a blank environment:

```bash
# Add the server to your agent config (Claude Desktop, Cursor, etc.)
# Either of:
#   command: "npx",  args: ["-y", "@printr/mcp@latest"]
#   command: "bunx", args: ["@printr/mcp@latest"]
```

Optional credentials, all read from environment variables:

```bash
export PRINTR_API_KEY="<partner-key>"                    # public default works for testing
export OPENROUTER_API_KEY="<your-openrouter-key>"        # enables printr_generate_image
export EVM_WALLET_PRIVATE_KEY="<hex>"                    # autonomous EVM signing
export SVM_WALLET_PRIVATE_KEY="<base58>"                 # autonomous Solana signing
export PRINTR_DEPLOYMENT_PASSWORD="$(openssl rand -base64 32)"  # required for treasury-protected launches
```

Verify the agent can reach the MCP server by calling `printr_supported_chains` — it should return a populated list of chains with CAIP-2 IDs.

## Commands

### Launch (one call)

`printr_launch_token` — create and sign in one call.

- **When to use**: the user asks to launch a token or deploy a new asset, and is happy with default signing behavior.
- **Output**: `token_id`, deployment payload, tx hashes per chain.
- **Example**: launch "Moon Cat" ($MCAT) on Solana with a $10 initial buy.
- **Signing**: omit `private_key` to open a browser signer via `printr_open_web_signer`; pass `private_key` (or set `EVM_WALLET_PRIVATE_KEY` / `SVM_WALLET_PRIVATE_KEY` in env) to sign autonomously.
- **Image**: omit `image` / `image_path` to auto-generate (requires `OPENROUTER_API_KEY`).

### Launch (two-step build + sign)

Use this flow when the agent should inspect or relay the unsigned payload before signing.

1. `printr_create_token` — returns an unsigned tx payload and token metadata.
2. `printr_sign_and_submit_evm` or `printr_sign_and_submit_svm` — sign and submit the payload (or pass `private_key` for autonomous mode).
3. `printr_open_web_signer` — open a browser session (MetaMask / Phantom) when no key is available.

### Treasury-protected launch

Production flow that keeps the treasury key offline behind an ephemeral deployment wallet.

1. `printr_set_treasury_wallet` — unlock the funding source for the session.
2. `printr_fund_deployment_wallet` — create and fund the ephemeral wallet (requires `PRINTR_DEPLOYMENT_PASSWORD`).
3. `printr_launch_token` — deploy using the active wallet automatically.
4. `printr_drain_deployment_wallet` — return unused funds to the treasury. Re-callable after MCP restart; recovers from persisted state.

### Cost estimation

`printr_quote` — call this before any launch and show the user the itemized costs.

- **Inputs**: chains, initial buy mode (pick ONE of `spend_usd`, `spend_native`, `supply_percent`).
- **Output**: per-chain itemized costs in native and USD plus expected initial buy amount.

### Wallet management

| Tool | Purpose |
|------|---------|
| `printr_wallet_new` | Generate an encrypted wallet |
| `printr_wallet_import` | Import an existing key |
| `printr_wallet_unlock` | Activate a stored wallet for the session |
| `printr_wallet_list` | List wallets (private keys hidden) |
| `printr_wallet_remove` | Remove a wallet from the keystore |
| `printr_wallet_bulk_remove` | Remove multiple wallets in one call |

### Utility tools

| Tool | Purpose |
|------|---------|
| `printr_get_balance` | Native token balance |
| `printr_get_token_balance` | ERC-20 / SPL balance |
| `printr_transfer` | Send native tokens |
| `printr_transfer_token` | Send ERC-20 / SPL tokens |
| `printr_get_token` | Token metadata by ID |
| `printr_get_deployments` | Per-chain deployment status |
| `printr_supported_chains` | List all chains with CAIP-2 IDs |
| `printr_generate_image` | Generate a token avatar via OpenRouter |

### Fees

| Tool | Purpose |
|------|---------|
| `printr_get_creator_fees` | Check claimable creator fees |
| `printr_claim_fees` | Claim accumulated fees to the treasury |

### Staking

| Tool | Purpose |
|------|---------|
| `printr_create_stake_position` | Open a stake position on a Printr token |
| `printr_get_staking_positions` | List stake positions (filter by token or owner) |
| `printr_claim_staking_rewards` | Claim rewards or withdraw unlocked principal |

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `Printr API error 401 / 403` | Missing or invalid `PRINTR_API_KEY` | Set a valid partner key or fall back to the documented public default |
| `Printr API error 500: chain is unsupported` | CAIP-2 chain ID not registered with Printr | Call `printr_supported_chains` and use one of the returned IDs |
| `Printr API error 500: token not found` | Unknown `token_id` passed to `printr_get_token` / `printr_get_deployments` | Verify the ID; if you just launched, allow a few seconds for indexing |
| `OPENROUTER_API_KEY not set` | Auto image generation requested without a key | Set `OPENROUTER_API_KEY` or pass `image` / `image_path` explicitly |
| `PRINTR_DEPLOYMENT_PASSWORD not set` | Treasury-protected flow invoked without the password | Generate one with `openssl rand -base64 32` and export it before calling `printr_fund_deployment_wallet` |
| `No active wallet` | Signing tool called without a key resolution path | Call `printr_wallet_unlock`, pass `private_key` explicitly, or set `EVM_WALLET_PRIVATE_KEY` / `SVM_WALLET_PRIVATE_KEY` |
| Browser signer never returns | User closed the tab or denied the request | Re-issue the signing call; the previous session is recoverable for a short window |
| Deployment wallet drain fails after restart | Lost session state | Re-run `printr_drain_deployment_wallet` — it recovers from persisted state using `PRINTR_DEPLOYMENT_PASSWORD` |

## Security Notices

- **Risk level**: `standard`. Every transaction requires user confirmation via the browser signer flow unless the user has explicitly set `EVM_WALLET_PRIVATE_KEY` or `SVM_WALLET_PRIVATE_KEY` (autonomous mode is opt-in per env-var, never plugin-driven).
- **Private keys**: never logged, never exfiltrated. Keys live either in the user's encrypted local keystore (AES-256-GCM, password-derived via scrypt), or in environment variables the user controls, or in the user's browser wallet — Printr's servers never see them.
- **`PRINTR_API_KEY`**: the hardcoded default is a documented public `ai-integration` key intended for testing. Production usage should set a partner key.
- **Treasury-protected flow**: `printr_fund_deployment_wallet` creates an ephemeral wallet so the user's treasury key is never exposed during a launch. Call `printr_drain_deployment_wallet` after every launch to recover unused funds.
- **Token risk**: launched tokens are speculative assets. The plugin does not provide investment advice; users are responsible for the tokens they create and the funds they commit.
- **Supported chains and RPCs**: declared in `api_calls` in `plugin.yaml`. The plugin makes no network calls outside that list.
