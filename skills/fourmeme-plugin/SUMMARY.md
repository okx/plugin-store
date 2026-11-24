## Overview

Trade and create memecoins on **Four.meme** - the largest bonding-curve token launchpad on BNB Smart Chain. Buy/sell pre-graduate tokens (still on Four.meme's internal AMM, before they migrate to PancakeSwap), launch new tokens (one-shot create with auto image upload + on-chain submit), query holdings (auto-discovered from Four.meme), and read trades / token rankings / TaxToken config from chain. All wallet signatures use OKX TEE wallets via `onchainos` (no raw private keys). Supports BNB chain (56) only; v0.1 buy/sell flow supports BNB-quoted tokens only, while create-token supports both BNB and USDT raised tokens.

## Prerequisites
- `onchainos` CLI installed and a BSC wallet active (`onchainos wallet addresses --chain 56` to verify)
- Wallet holds BNB on BSC for gas (typical buy: 200k gas at 0.1 gwei is ~$0.001; create-token: 3-4M gas, ~$0.01 at current prices)
- For cookie-gated commands (`create-token`, `positions` auto-mode): a Four.meme session token. `quickstart` auto-creates one on first run via SIWE wallet signature.

## Quick Start
1. Check your wallet and provision auth in one step: `fourmeme-plugin quickstart`
2. If you see `status: chain_invalid` - re-run with the right chain: `fourmeme-plugin quickstart --chain 56`
3. If you see `status: no_wallet` - create a wallet: `onchainos wallet add`, then re-run `fourmeme-plugin quickstart`
4. If you see `status: no_funds` - top up BNB on BSC, then re-run `fourmeme-plugin quickstart`
5. If you see `status: ready_to_trade` - browse trending tokens: `fourmeme-plugin list-tokens --type HOT --limit 10`, then `fourmeme-plugin quote-buy --token 0x... --funds 0.01`, then `fourmeme-plugin buy --token 0x... --funds 0.01`
6. If you see `status: active` - review your full holdings (auto-discovered): `fourmeme-plugin positions`
7. To exit a position: `fourmeme-plugin sell --token 0x... --all`
8. To launch a new token in one shot: `fourmeme-plugin create-token --name "MyMeme" --symbol "MM" --image-file ./logo.png --quote bnb --label AI --desc "..."`
