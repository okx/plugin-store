---
title: Token Prices and Market Data
impact: HIGH
impactDescription: "Price verification before bridge/swap operations"
tags: prices, market-data, coingecko, cryptocom, ohlcv, trending, orderbook
---

# Token Prices and Market Data

Real-time and historical token pricing. Use before deBridge operations to verify token value, check price impact, or compare across chains.

## CoinGecko MCP (Primary — No API Key)

CoinGecko covers 15,000+ coins across 200+ networks including on-chain DEX data via GeckoTerminal.

### Installation

Hosted endpoint (no install, no key):

```bash
claude mcp add --transport http coingecko https://mcp.api.coingecko.com/mcp
```

Local (one-shot): `npx -y @coingecko/coingecko-mcp`

### Key Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `get_simple_price` | `vs_currencies`* (string), `ids` (string), `symbols` (string), `names` (string), `include_market_cap`, `include_24hr_vol`, `include_24hr_change`, `include_last_updated_at`, `precision` | Current prices with optional market cap and volume |
| `get_id_coins` | `id`* (coin ID), `market_data`, `localization`, `tickers`, `community_data`, `developer_data`, `sparkline` | Full coin data: description, links, market data |
| `get_coins_markets` | `vs_currency`* (e.g., "usd"), `ids`, `symbols`, `names`, `category`, `order`, `per_page`, `page`, `sparkline`, `price_change_percentage`, `precision` | Paginated market data for multiple coins |
| `get_search` | `query`* (string) | Search coins, categories, and exchanges by keyword |
| `get_search_trending` | `show_max` (string) | Trending coins and NFTs |
| `get_id_simple_token_price` | `id`* (platform), `contract_addresses`* (string), `vs_currencies`* (string), `include_market_cap`, `include_24hr_vol`, `include_24hr_change`, `include_last_updated_at`, `precision` | Token price by contract address on a platform |
| `get_coins_contract` | `id`* (platform), `contract_address`* | Coin data by contract address |
| `get_range_coins_market_chart` | `id`*, `vs_currency`*, `from`*, `to`*, `interval`, `precision` | Historical OHLCV in a date range |
| `get_range_coins_ohlc` | `id`*, `vs_currency`*, `from`*, `to`*, `interval`* | OHLC candlestick data in a date range |
| `get_tokens_networks_onchain_info` | `network`*, `address`* | On-chain token info via GeckoTerminal |

### CoinGecko Coin IDs

CoinGecko uses slug-style IDs (not ticker symbols):
- ETH → `ethereum`
- USDC → `usd-coin`
- SOL → `solana`

Call `get_search` or `get_id_coins` to resolve a symbol to its CoinGecko ID.

### Example: Check Price Before Bridging

```
1. Call mcp__coingecko__get_simple_price:
     ids: "usd-coin"
     vs_currencies: "usd"
     include_24hr_change: true

2. Verify USDC is at expected peg ($1.00 ± 0.01).

3. If stable → proceed to ../swap/SKILL.md.
   If depegged → warn the user before proceeding.
```

---

## Crypto.com Exchange MCP (No API Key — Hosted)

Real-time exchange data from Crypto.com: orderbooks, recent trades, and OHLCV candles. Complements CoinGecko by providing exchange-level market microstructure data.

### Installation

Hosted endpoint (no install, no key):

```bash
claude mcp add --transport http cryptocom https://mcp.crypto.com/market-data/mcp
```

### Key Tools

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `get_instruments` | (none) | List all available trading instruments |
| `get_instrument` | `instrument_name`\* | Instrument detail by ID |
| `get_tickers` | `instrument_name` | Ticker(s) — price, volume, bid/ask for one or all instruments |
| `get_ticker` | `instrument_name`\* | Single ticker for an instrument |
| `get_index_price` | `instrument_name`\* | Index price for an instrument |
| `get_mark_price` | `instrument_name`\* | Mark price for an instrument |
| `get_book` | `instrument_name`\*, `depth` (max 150) | Order book snapshot (bids/asks) |
| `get_trades` | `instrument_name`\*, `count` (max 150) | Recent trades (default 10) |
| `get_candlestick` | `instrument_name`\*, `timeframe`\* | OHLCV candles (up to 50) |

Instrument names use underscore format: `BTC_USDT`, `ETH_USDT`, `SOL_USDT`, etc. Call `get_instruments` to list all.

### Example: Check BTC Market Depth Before Large Swap

```
1. Call mcp__cryptocom__get_book:
     instrument_name: "BTC_USDT"
     depth: 20

2. Review bid/ask spread and depth to estimate slippage.

3. Call mcp__cryptocom__get_candlestick:
     instrument_name: "BTC_USDT"
     timeframe: "4h"

4. Check recent price trend before proceeding with the bridge.
```

---

## mcp-crypto-price (No API Key — Local)

Real-time prices, market analysis with exchange volume distribution, and historical trend analysis. Uses the CoinCap public API.

### Installation

```bash
claude mcp add crypto-price -- npx -y mcp-crypto-price
```

One-shot: `npx -y mcp-crypto-price`

### Key Tools

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `get-crypto-price` | `symbol`\* (e.g., "BTC") | Real-time price, 24h change, volume, market cap |
| `get-market-analysis` | `symbol`\* | Top 5 exchanges by volume with price per exchange and volume distribution % |
| `get-historical-analysis` | `symbol`\*, `interval` (m1/m5/m15/m30/h1/h2/h6/h12/d1), `days` (1-30) | Historical data with trend analysis, high/low, volatility metrics |
| `get-top-assets` | `limit` (1-50, default 10) | Top cryptocurrencies ranked by market cap |

### Example: Pre-Bridge Market Analysis

```
1. Call mcp__crypto_price__get-market-analysis:
     symbol: "ETH"

2. Review which exchanges have the most volume and
   whether prices are consistent across them.

3. Call mcp__crypto_price__get-historical-analysis:
     symbol: "ETH"
     interval: "h1"
     days: 7

4. Check volatility metrics before committing to the swap.
```

---

## When to Use Which

| Scenario | MCP | Tool |
|----------|-----|------|
| Quick price check by coin name | CoinGecko | `get_simple_price` |
| Price by contract address | CoinGecko | `get_coins_contract` or `get_id_simple_token_price` |
| Historical OHLCV chart (by coin ID) | CoinGecko | `get_range_coins_market_chart` |
| On-chain DEX token data | CoinGecko | `get_tokens_networks_onchain_info` (GeckoTerminal) |
| Trending coins and markets | CoinGecko | `get_search_trending` |
| Exchange orderbook depth | Crypto.com | `get_book` |
| Exchange-level OHLCV candles | Crypto.com | `get_candlestick` |
| Recent exchange trades | Crypto.com | `get_trades` |
| Exchange volume distribution | mcp-crypto-price | `get-market-analysis` |
| Historical volatility and trends | mcp-crypto-price | `get-historical-analysis` |
