# Birdeye Plugin Summary

## Overview
Birdeye plugin provides DeFi analytics endpoint access in dual mode: API key and x402.

## Prerequisites
- API key mode: set `BIRDEYE_API_KEY`
- x402 mode: set `SOLANA_PRIVATE_KEY` (base58) and ensure USDC balance on Solana mainnet
- Node.js 20+ is required for x402 runtime

## Quick Start
1. Set `BIRDEYE_MODE` and credentials in environment.
2. Build runtime in `runtime/`.
3. List endpoints: `node dist/index.js list --mode apikey|x402`.
4. Call endpoint: `node dist/index.js call --endpoint <key> --chain solana ...`.
