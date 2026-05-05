---
name: birdeye-plugin
description: Birdeye DeFi analytics with dual access architecture (apikey active, x402 scaffold). Use this skill for price, trending, token overview, and security queries.
---

# Birdeye Plugin Skill

This skill routes user intent to the local runtime CLI.

Current status:
- `apikey` mode: active
- `x402` mode: scaffolded in runtime contract, payment signer implementation pending
- `auto` mode: currently resolves effectively to `apikey` path for live calls

## Environment

- `BIRDEYE_MODE=auto|apikey|x402` (default: `auto`)
- `BIRDEYE_API_KEY=...` (required for live API calls)
- `SOLANA_PRIVATE_KEY='[1,2,3,...]'` (reserved for upcoming x402 signer integration)

## Commands

- `node runtime/dist/index.js price --address <TOKEN> --chain solana`
- `node runtime/dist/index.js trending --chain solana --limit 20`
- `node runtime/dist/index.js overview --address <TOKEN> --chain solana`
- `node runtime/dist/index.js security --address <TOKEN> --chain solana`

## Intent Mapping

- Price now -> `price`
- Trending tokens -> `trending`
- Token overview -> `overview`
- Token risk/security -> `security`

## Notes

- Standard path uses `https://public-api.birdeye.so`
- x402 path target is `https://public-api.birdeye.so/x402` (not enabled for live signing yet)
- Wallet endpoints should remain on API key mode
