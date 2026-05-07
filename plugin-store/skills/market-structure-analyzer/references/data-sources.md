# Data Sources Reference v2.0

## Priority: OKX First, Binance Fallback

### OKX Public (PRIMARY)
Base: `https://www.okx.com/api/v5/`

| Endpoint | Data | Notes |
|----------|------|-------|
| `public/funding-rate?instId=BTC-USDT-SWAP` | Current funding rate | Returns `fundingRate`, `fundingTime`, `nextFundingRate` |
| `public/funding-rate-history?instId=BTC-USDT-SWAP&limit=6` | Funding history (48h) | 6 periods x 8h = 48h trend |
| `public/open-interest?instType=SWAP&instId=BTC-USDT-SWAP` | Swap OI (contracts + currency) | `oi` = contracts, `oiCcy` = coin-denominated |
| `public/open-interest?instType=OPTION&instFamily=BTC-USD` | Per-instrument option OI | ~1690 records, ~547 non-zero. Used for gamma wall |
| `public/opt-summary?instFamily=BTC-USD` | Options Greeks + IV | ~736 instruments. **Only source with gammaBS, deltaBS, markVol > 0** |
| `market/ticker?instId=BTC-USDT` | Spot ticker | price, 24h change, volume |
| `market/ticker?instId=BTC-USDT-SWAP` | Swap ticker | Used for basis calc (swap - spot) |
| `market/candles?instId=BTC-USDT&bar=1H&limit=24` | Hourly candles | Used for realized volatility (log returns) |
| `rubik/stat/taker-volume?ccy=BTC&instType=CONTRACTS&period=1H` | Taker buy/sell volume | Returns arrays `[ts, sellVol, buyVol]` — **sell first, buy second** |
| `rubik/stat/contracts/open-interest-volume?ccy=BTC&period=6H` | OI history | Returns arrays `[ts, oi, vol]` (not dicts). Used for OI delta |

**IMPORTANT OKX Notes:**
- Always check `code != "0"` in response — OKX returns HTTP 200 even on errors
- `/market/tickers?instType=OPTION` returns **zero** for all Greeks — do NOT use for gamma/skew
- `taker-volume-contract` endpoint returns 400 — use `rubik/stat/taker-volume` instead
- Rate limit: 20 req/2s per endpoint. Add 100ms delay between calls

### Binance Futures Public (FALLBACK)
Base: `https://fapi.binance.com/`

| Endpoint | Data | Used When |
|----------|------|-----------|
| `fapi/v1/fundingRate?symbol=BTCUSDT&limit=1` | Funding rate | Cross-reference + OKX funding fails |
| `fapi/v1/openInterest?symbol=BTCUSDT` | Open interest | Cross-exchange OI comparison |
| `futures/data/topLongShortPositionRatio?symbol=BTCUSDT&period=1h&limit=1` | Top trader L/S | OKX L/S fails |
| `futures/data/globalLongShortAccountRatio?symbol=BTCUSDT&period=1h&limit=1` | Global L/S | OKX L/S fails |
| `futures/data/takerlongshortRatio?symbol=BTCUSDT&period=1h&limit=1` | Taker ratio | Binance fallback taker ratio |
| `futures/data/globalLongShortAccountRatio?symbol=BTCUSDT&period=1h&limit=12` | L/S ratio history (12h) | **Liquidation pressure proxy** — measures L/S swing |

**IMPORTANT Binance Notes:**
- `allForceOrders` endpoint is **deprecated** ("out of maintenance", returns 400). Do NOT use.
- Use `globalLongShortAccountRatio` history as liquidation proxy instead.
- Rate limit: 1200 req/min — plenty, no throttle needed

### CoinMetrics Community API (FREE, no auth)
Base: `https://community-api.coinmetrics.io/v4/`

| Endpoint | Data | Notes |
|----------|------|-------|
| `timeseries/asset-metrics?assets=btc&metrics=CapMVRVCur&frequency=1d&start_time=<30d_ago>&page_size=60` | MVRV ratio (30d history) | `CapMVRVCur` is free tier. `CapRealUSD` is premium (403) |

**IMPORTANT CoinMetrics Notes:**
- Use `page_size` not `limit` (limit is not supported)
- `sort=desc` is not supported — data comes oldest-first
- Realized price is derived: `spot_price / mvrv`
- Supports `btc` and `eth` (lowercase)

### Alternative.me
- Fear & Greed: `https://api.alternative.me/fng/?limit=7`
- Returns: `{ data: [{ value: "12", value_classification: "Extreme Fear", timestamp: "..." }] }`
- 7-day history for trend analysis

### CoinGecko (free tier, 10-30 req/min)
Base: `https://api.coingecko.com/api/v3/`

| Endpoint | Data |
|----------|------|
| `global` | BTC dominance, ETH dominance, total market cap, 24h volume, market cap change |

### DefiLlama
- Stablecoins: `https://stablecoins.llama.fi/stablecoins?includePrices=true`
- Returns top stablecoins by market cap (USDT, USDC, USDS, USDe, DAI, etc.)

## Token Symbol Mapping

| Token | OKX Swap | OKX Spot | OKX Options Family | Binance | CoinGecko ID | Tier |
|-------|----------|----------|-------------------|---------|--------------|------|
| BTC | BTC-USDT-SWAP | BTC-USDT | BTC-USD | BTCUSDT | bitcoin | 1 |
| ETH | ETH-USDT-SWAP | ETH-USDT | ETH-USD | ETHUSDT | ethereum | 1 |
| SOL | SOL-USDT-SWAP | SOL-USDT | SOL-USD | SOLUSDT | solana | 2 |
| BNB | BNB-USDT-SWAP | BNB-USDT | — | BNBUSDT | binancecoin | 2 |
| AVAX | AVAX-USDT-SWAP | AVAX-USDT | — | AVAXUSDT | avalanche-2 | 2 |
| DOGE | DOGE-USDT-SWAP | DOGE-USDT | — | DOGEUSDT | dogecoin | 2 |
| ARB | ARB-USDT-SWAP | ARB-USDT | — | ARBUSDT | arbitrum | 2 |

## Rate Limits

| API | Limit | Strategy |
|-----|-------|----------|
| OKX | 20 req/2s per endpoint | Add 100ms delay between calls |
| Binance | 1200 req/min | Plenty, no throttle needed |
| CoinMetrics | ~100 req/min (community) | Fetch once per analysis |
| CoinGecko Free | 10-30 req/min | Fetch once per analysis |
| Alternative.me | No documented limit | Fetch once per analysis |
| DefiLlama | No documented limit | Fetch once per analysis |

## Dune Analytics (requires MCP tools)

Access: Via Dune MCP tools (`mcp__dune__executeQueryById`, `mcp__dune__getExecutionResults`)
Key table: `cex.flows` (Spellbook spell) — tracks token deposits/withdrawals to labeled CEX wallets

### Pre-built Queries

| Query ID | Name | SQL Summary | Typical Cost |
|----------|------|-------------|--------------|
| **6988944** | ETH CEX Net Flows (7d) | Daily `SUM(deposit) - SUM(withdrawal)` for ETH from `cex.flows` grouped by `DATE(block_time)` | 0.025 credits |
| **6988945** | CEX Flows by Exchange (24h) | Per-exchange net flows for ETH + stablecoins, filtered to >$100K net, last 24h | 0.01 credits |
| **6988947** | Whale ETH Transfers (24h) | `tokens.transfers` JOIN `cex.addresses` for ETH transfers >100 ETH, classified as CEX deposit/withdrawal/wallet-to-wallet | 0.079 credits |
| **6988949** | Stablecoin CEX Flows (7d) | Daily USDT + USDC net flows from `cex.flows`, last 7 days | 0.018 credits |

### Dune Table Reference

| Table | Description | Key Columns |
|-------|-------------|-------------|
| `cex.flows` | Token flows into/out of CEX wallets | `block_time`, `cex_name`, `token_symbol`, `flow_type` ('deposit'/'withdrawal'), `amount`, `amount_usd` |
| `cex.addresses` | Known CEX wallet addresses | `blockchain`, `address`, `cex_name`, `distinct_name` |
| `tokens.transfers` | All ERC20 + native transfers | `block_time`, `block_date`, `from`, `to`, `symbol`, `amount`, `amount_usd` |

**IMPORTANT**: `cex.flows` does NOT have a `block_date` column. Use `DATE(block_time)` for daily grouping.

### Execution Pattern

```
1. executeQueryById(query_id=6988944)  →  execution_id_1
2. executeQueryById(query_id=6988945)  →  execution_id_2
3. executeQueryById(query_id=6988947)  →  execution_id_3
4. executeQueryById(query_id=6988949)  →  execution_id_4
   (all 4 in parallel)

5. getExecutionResults(execution_id_1, timeout=120)
6. getExecutionResults(execution_id_2, timeout=120)
7. getExecutionResults(execution_id_3, timeout=120)
8. getExecutionResults(execution_id_4, timeout=120)
   (all 4 in parallel)
```

## Fallback Chain

For each data type, try sources in order:

1. **Funding Rate:** OKX → Binance
2. **Funding History (48h):** OKX (no fallback)
3. **Open Interest:** OKX → Binance (for cross-exchange comparison)
4. **OI History / Delta:** OKX rubik (no fallback)
5. **Taker Volume:** OKX rubik (no fallback)
6. **Long/Short Ratio:** OKX → Binance (top trader + global)
7. **Futures Basis:** OKX (swap - spot, no fallback)
8. **Gamma Wall:** OKX opt-summary + option OI (Tier 1 only, no fallback)
9. **25-Delta Skew:** OKX opt-summary (Tier 1 only, no fallback)
10. **MVRV:** CoinMetrics community API (BTC + ETH only, no fallback)
11. **Liquidation Pressure:** Binance L/S ratio history swing analysis (no fallback)
12. **Realized Volatility:** OKX hourly candles (no fallback)
13. **Exchange Flows:** Dune `cex.flows` (optional, requires MCP tools)
14. **Whale Transfers:** Dune `tokens.transfers` + `cex.addresses` (optional)
15. **Stablecoin Exchange Flows:** Dune `cex.flows` filtered to USDT/USDC (optional)
16. **Sentiment:** Alternative.me (sole source)
17. **Stablecoin Market Cap:** DefiLlama (sole source)
18. **BTC Dominance / Market Cap:** CoinGecko global endpoint
