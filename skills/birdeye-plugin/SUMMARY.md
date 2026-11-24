# Birdeye Plugin Summary

## Overview
Birdeye plugin provides DeFi analytics endpoint access in dual mode: API key and x402.

## Prerequisites
- API key mode: set `BIRDEYE_API_KEY`
- x402 mode: set key file at `~/.birdeye/key` (base58, chmod 600) and ensure USDC balance on Solana mainnet
- Node.js 20+ is required for x402 runtime

## Quick Start
1. Run quickstart check: `node ./runtime/dist/index.js list --mode apikey` after exporting `BIRDEYE_API_KEY`.
2. Build runtime in `runtime/`.
3. List endpoints: `node dist/index.js list --mode apikey|x402`.
4. Call endpoint: `node dist/index.js call --endpoint <key> --chain solana ...`.
