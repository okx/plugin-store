---
name: birdeye-plugin
description: Birdeye DeFi analytics with dual live access mode (apikey and x402). Use this skill for price, trending, token overview, and security queries.
---

# Birdeye Plugin Skill

This skill routes user intent to the local runtime CLI.

## Runtime Requirement Notes

- `apikey` mode can run on lower Node versions.
- `x402` mode requires Node.js 20+.
- If `BIRDEYE_MODE=x402` (or `auto` resolves to x402), verify `node -v` first and ensure major version is `>= 20`.
- If Node is below 20, switch/install Node 20 before running x402 requests.

Supported live modes:
- `apikey` mode: standard Birdeye REST (`X-API-KEY`)
- `x402` mode: Birdeye pay-per-request (`/x402`) with Solana USDC payment signing
- `auto` mode: prefer `apikey`, fallback to `x402`

## Environment

- `BIRDEYE_MODE=auto|apikey|x402` (default: `auto`)
- `BIRDEYE_API_KEY=...` (required for apikey mode)
- `SOLANA_PRIVATE_KEY=...` (base58 private key, required for x402 mode)

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

## Operational Notes

- Standard mode target: `https://public-api.birdeye.so`
- x402 mode target: `https://public-api.birdeye.so/x402`
- x402 requests require wallet balance (USDC on Solana mainnet) and valid signing credentials.
- If an endpoint is unavailable in x402, switch to `apikey` mode.
