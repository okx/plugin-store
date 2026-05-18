---
title: DEX Pool Analytics — Liquidity, OHLCV, Trades
impact: MEDIUM
impactDescription: "Pool liquidity and price data for swap destination assessment"
tags: dexpaprika, dex, pools, ohlcv, liquidity, trades, coinpaprika
---

# DEX Pool Analytics

DexPaprika provides real-time DEX pool data: liquidity, OHLCV candles, recent trades, and token info across multiple networks. No API key required. Use before deBridge swaps to assess destination pool depth and recent trading activity.

## Installation

Hosted endpoint (recommended — no install):

```bash
claude mcp add --transport http dexpaprika https://mcp.dexpaprika.com/streamable-http
```

Local (alternative): `npx dexpaprika-mcp` — no API key needed.

## Key Tools

### Discovery

| Tool | Parameters (* = required) | Description |
|------|-----------|-------------|
| `getCapabilities` | — | Server capabilities, workflow patterns, network synonyms, common pitfalls |
| `getNetworks` | — | All supported networks |
| `getNetworkDexes` | `network`*, `page`, `limit`, `sort`(asc/desc), `order_by`(pool) | DEXes on a specific network |
| `search` | `query`* | Search tokens, pools, DEXes by name/symbol/address |
| `getStats` | — | Aggregate DEX statistics |

### Pool Data

| Tool | Parameters (* = required) | Description |
|------|-----------|-------------|
| `getNetworkPools` | `network`*, `page`, `limit`, `sort`(asc/desc), `order_by`(volume_usd/price_usd/transactions/last_price_change_usd_24h/created_at) | All pools on a network |
| `getDexPools` | `network`*, `dex`*, `page`, `limit`, `sort`(asc/desc), `order_by`(volume_usd/price_usd/transactions/last_price_change_usd_24h/created_at) | Pools for a specific DEX |
| `getNetworkPoolsFilter` | `network`*, `page`, `limit`, `volume_24h_min`, `volume_24h_max`, `txns_24h_min`, `created_after`(unix), `created_before`(unix), `sort_by`(volume_24h/txns_24h/created_at), `sort_dir`(asc/desc) | Filtered pool search |
| `getPoolDetails` | `network`*, `pool_address`*, `inversed` | Full pool info: TVL, volume, fees |
| `getPoolOHLCV` | `network`*, `pool_address`*, `start`*, `end`, `limit`, `interval`(1m/5m/10m/15m/30m/1h/6h/12h/24h), `inversed` | Candlestick price data |
| `getPoolTransactions` | `network`*, `pool_address`*, `page`, `limit`, `cursor` | Recent swaps and trades |

### Token Data

| Tool | Parameters (* = required) | Description |
|------|-----------|-------------|
| `getTokenDetails` | `network`*, `token_address`* | Token metadata and market data |
| `getTokenPools` | `network`*, `token_address`*, `page`, `limit`, `sort`(asc/desc), `order_by`(volume_usd/price_usd/transactions/last_price_change_usd_24h/created_at), `reorder`, `address` | Pools containing a token |
| `getTokenMultiPrices` | `network`*, `tokens`*: array (up to 10) | Batch token prices |

## Supported Networks

DexPaprika uses network slug identifiers:

| Network | Slug |
|---------|------|
| Ethereum | `ethereum` |
| Arbitrum | `arbitrum` |
| Base | `base` |
| Polygon | `polygon` |
| BNB Chain (BSC) | `bsc` |
| Optimism | `optimism` |
| Avalanche | `avalanche` |
| Solana | `solana` |

Call `getNetworks` for the full list.

## Example: Check Liquidity Before Cross-Chain Swap

```
1. Search for the destination token:
   Call mcp__dexpaprika__search:
     query: "USDC base"

2. Get pools containing the token:
   Call mcp__dexpaprika__getTokenPools:
     network: "base"
     token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
     order_by: "volume_24h"
     limit: 5

3. Check the top pool's details:
   Call mcp__dexpaprika__getPoolDetails:
     network: "base"
     pool_address: "<pool-address-from-step-2>"

4. If 24h volume > bridge amount and TVL is healthy →
   proceed with the swap.
   If low liquidity → warn the user about potential slippage.
```

## Example: Get Price Candles for a Pool

```
Call mcp__dexpaprika__getPoolOHLCV:
  network: "ethereum"
  pool_address: "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640"  // USDC/ETH on Uniswap V3
  start: "2025-01-01"
  interval: "1d"
  limit: 30

Returns 30 daily candles with open, high, low, close, and volume.
```
