---
title: Supported Chains and Token Configuration
impact: HIGH
impactDescription: "Required for all cross-chain operations — chain IDs, tokens, decimals"
tags: chains, tokens, config, chain-id, decimals, amounts
---

# Chain Configuration

Use `mcp__debridge__get_supported_chains` for the live list. This reference covers common chains for quick lookup.

## EVM Chains

| Chain             | deBridge Chain ID | Native Token | Decimals |
|-------------------|-------------------|--------------|----------|
| Ethereum          | 1                 | ETH          | 18       |
| BNB Chain         | 56                | BNB          | 18       |
| Polygon           | 137               | POL          | 18       |
| Arbitrum One      | 42161             | ETH          | 18       |
| Optimism          | 10                | ETH          | 18       |
| Avalanche C-Chain | 43114             | AVAX         | 18       |
| Base              | 8453              | ETH          | 18       |
| Linea             | 59144             | ETH          | 18       |
| Sonic             | 100000014         | S            | 18       |
| Berachain         | 100000020         | BERA         | 18       |
| Mantle            | 100000023         | MNT          | 18       |
| Abstract          | 100000017         | ETH          | 18       |

Native token address on all EVM chains: `0x0000000000000000000000000000000000000000`

## Non-EVM Chains

| Chain  | deBridge Chain ID | Native Token | Decimals | Native Address                             |
|--------|-------------------|--------------|----------|--------------------------------------------|
| Solana | 7565164           | SOL          | 9        | 11111111111111111111111111111111             |
| Tron   | 100000026         | TRX          | 6        | T9yD14Nj9j7xAB4dbGeiX9h8unkKHxuWwb        |

## deBridge-Internal Chain IDs

Some chains use deBridge-internal IDs that differ from standard EVM chain IDs. Always use the deBridge chain ID when calling MCP tools.

| Chain     | deBridge ID | Standard Chain ID |
|-----------|-------------|-------------------|
| Neon      | 100000001   | 245022934         |
| Gnosis    | 100000002   | 100               |
| Sonic     | 100000014   | 146               |
| Abstract  | 100000017   | 2741              |
| Berachain | 100000020   | 80094             |
| Mantle    | 100000023   | 5000              |

## Common Tokens

Use `mcp__debridge__search_tokens` for the live database. Quick reference:

### USDC

| Chain    | Address                                      | Decimals |
|----------|----------------------------------------------|----------|
| Ethereum | 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48  | 6        |
| Arbitrum | 0xaf88d065e77c8cC2239327C5EDb3A432268e5831  | 6        |
| Polygon  | 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359  | 6        |
| Base     | 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913  | 6        |
| Solana   | EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | 6       |

### USDT

| Chain     | Address                                      | Decimals |
|-----------|----------------------------------------------|----------|
| Ethereum  | 0xdAC17F958D2ee523a2206206994597C13D831ec7  | 6        |
| BNB Chain | 0x55d398326f99059fF775485246999027B3197955  | 18       |
| Arbitrum  | 0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9  | 6        |
| Tron      | TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t          | 6        |

## Amount Conversion

All deBridge MCP tools expect amounts in **raw units** (the smallest indivisible unit of a token: wei for ETH, lamports for SOL) passed as strings.

Formula: `raw_units = human_amount × 10^decimals`

| Human Amount | Token | Decimals | Raw Units                |
|--------------|-------|----------|--------------------------|
| "100"        | USDC  | 6        | "100000000"              |
| "1.5"        | ETH   | 18       | "1500000000000000000"    |
| "0.5"        | SOL   | 9        | "500000000"              |
| "50"         | USDT  | 6        | "50000000"               |

Always look up token decimals first via `mcp__debridge__search_tokens` if the token is not listed above.

### Bundled Script

The `scripts/convert-amount.ts` helper handles conversion in both directions and can read decimals on-chain:

```bash
npx tsx scripts/convert-amount.ts 100 6                          # → {"raw":"100000000","decimals":6,"human":"100"}
npx tsx scripts/convert-amount.ts 100 0xA0b8...eB48 1            # reads decimals from contract on Ethereum
npx tsx scripts/convert-amount.ts 100000000 6 --reverse           # raw → human
```

## Dynamic Lookup

For tokens not listed:
1. Call `mcp__debridge__search_tokens` with the token name, symbol, or address.
2. Response includes `address`, `decimals`, `chainId`, and `symbol`.
3. Use the returned values for subsequent MCP calls.

For chains not listed:
1. Call `mcp__debridge__get_supported_chains` (no parameters).
2. Response includes all supported chain IDs and names.
