---
title: DeFi Protocol Analytics — TVL, Fees, Yields, Vault Risk
impact: HIGH
impactDescription: "Protocol health, yield data, and vault risk scoring for informed bridging decisions"
tags: defillama, philidor, tvl, fees, revenue, yields, stablecoins, dex-volume, vault-risk
---

# DeFi Protocol Analytics

DefiLlama provides protocol TVL, fee revenue, yield farming data, DEX volumes, stablecoin metrics, and token prices — all without an API key.

## Installation

Two npm packages are available. Both wrap the DefiLlama public API.

### @nic0xflamel/defillama-mcp-server (OpenAPI proxy)

Dynamically generates tools from the full DefiLlama OpenAPI surface.

```bash
claude mcp add defillama -- npx -y @nic0xflamel/defillama-mcp-server
```

### @iqai/defillama-mcp (curated tools with AI entity resolution)

19 typed tools with fuzzy matching on protocol/chain names. Includes token pricing endpoints not in @nic0xflamel.

```bash
claude mcp add defillama -- pnpm dlx @iqai/defillama-mcp
```

No API key required for either package. Tool names differ — see sections below.

---

## Key Tools (@nic0xflamel/defillama-mcp-server)

### TVL and Protocols

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `get_v2_chains` | (none) | Current TVL of all chains |
| `get_protocols` | (none) | List all protocols with TVL |
| `get_tvl__by_protocol` | `protocol`\* | Current TVL of a specific protocol |
| `get_v2_historicalChainTvl` | (none) | Historical TVL of DeFi on all chains |
| `get_v2_historicalChainTvl__by_chain` | `chain`\* | Historical TVL of a specific chain |

### DEX Volumes

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `get_overview_dexs` | `excludeTotalDataChart`, `excludeTotalDataChartBreakdown` | DEX volume summaries across all chains |
| `get_overview_dexs__by_chain` | `chain`\*, `excludeTotalDataChart`, `excludeTotalDataChartBreakdown` | DEX volumes for a specific chain |
| `get_summary_dexs__by_protocol` | `protocol`\*, `excludeTotalDataChart`, `excludeTotalDataChartBreakdown` | DEX volume for a specific protocol |

### Fees and Revenue

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `get_overview_fees` | `excludeTotalDataChart`, `excludeTotalDataChartBreakdown`, `dataType` (dailyFees/dailyRevenue) | Fee and revenue summaries |
| `get_overview_fees__by_chain` | `chain`\*, `excludeTotalDataChart`, `excludeTotalDataChartBreakdown`, `dataType` | Fees and revenue for a specific chain |
| `get_summary_fees__by_protocol` | `protocol`\*, `dataType` (dailyFees/dailyRevenue) | Fees and revenue for a specific protocol |

### Yields

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `get_pools` | (none) | Latest data for all yield pools with predictions |
| `get_chart__by_pool` | `pool`\* (UUID from pool data) | Historical APY and TVL for a specific pool |

### Stablecoins

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `get_stablecoincharts_all` | `stablecoin` (integer — optional filter) | Historical market cap sum of all stablecoins |

> **Note:** @nic0xflamel does not include token pricing tools. Use @iqai (below) or CoinGecko MCP for pricing.

---

## Key Tools (@iqai/defillama-mcp)

All @iqai tools support AI entity resolution — pass protocol/chain names as-is (e.g., "Uniswap", "Ethereum") and they auto-resolve to slugs.

### TVL and Protocols

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `defillama_get_chains` | `order` (asc/desc) | Chains ranked by TVL (top 20) |
| `defillama_get_protocol_data` | `protocol`, `sortCondition` (change_1h/1d/7d/tvl), `order` | Protocol TVL; omit protocol for top 10 |
| `defillama_get_historical_chain_tvl` | `chain` | Historical TVL over time (last 10 points) |

### DEX Volumes

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `defillama_get_dexs_data` | `protocol`, `chain`, `sortCondition` (total24h/total7d/total30d/change_1d/change_7d/change_1m), `order` | DEX trading volume metrics |

### Fees and Revenue

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `defillama_get_fees_and_revenue` | `protocol`, `chain`, `dataType` (dailyFees/dailyRevenue/dailyHoldersRevenue), `sortCondition`, `order` | Protocol fee and revenue metrics |

### Yields

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `defillama_get_latest_pool_data` | `sortCondition` (tvlUsd/apy/apyBase/apyReward/apyMean30d), `order`, `limit` (1-100) | Current yield farming pools with APY |
| `defillama_get_historical_pool_data` | `pool`\* (UUID from latest pool data) | Historical APY/TVL for a specific pool |

### Stablecoins

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `defillama_get_stablecoin` | `includePrices` (boolean) | Top 20 stablecoins with circulation |
| `defillama_get_stablecoin_chains` | (none) | Chains ranked by stablecoin market cap |
| `defillama_get_stablecoin_charts` | `stablecoin` (ID or name), `chain` | Historical stablecoin market cap |
| `defillama_get_stablecoin_prices` | (none) | Historical stablecoin price data |

### Token Prices (only in @iqai)

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `defillama_get_prices_current_coins` | `coins`\* ("chain:address"), `searchWidth` | Current token prices by contract |
| `defillama_get_chart_coins` | `coins`\*, `start`, `end`, `span`, `period`, `searchWidth` | Historical price time-series |
| `defillama_get_percentage_coins` | `coins`\*, `timestamp`, `period` (1h/1d/7d), `lookForward` | Price change percentage |
| `defillama_get_historical_prices_by_contract` | `coins`\*, `timestamp`\*, `searchWidth` | Historical prices at a specific time |
| `defillama_get_batch_historical` | `coins`\*, `searchWidth` | Historical prices at multiple timestamps |
| `defillama_get_prices_first_coins` | `coins`\* | First recorded price for tokens |

### Other (only in @iqai)

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `defillama_get_options_data` | `dataType`, `protocol`, `chain`, `sortCondition`, `order` | Options protocol volume and premiums |
| `defillama_get_blockchain_timestamp` | `chain`\*, `timestamp`\* | Block number at a specific time |

---

## Example: Research Before a Large Bridge

```
1. Check deBridge TVL → get_tvl__by_protocol (protocol: "debridge")
   or defillama_get_protocol_data (protocol: "debridge")

2. Check yield on destination chain → get_pools or defillama_get_latest_pool_data
   (sortCondition: "apy", order: "desc", limit: 10)

3. Verify token price (@iqai only) → defillama_get_prices_current_coins
   (coins: "ethereum:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")

4. Use findings to advise on best chain/token for yield or liquidity.
```

Chain slugs for pricing: `ethereum`, `bsc`, `polygon`, `arbitrum`, `optimism`, `base`, `avalanche`, `solana`.

For token pricing without @iqai, use CoinGecko MCP (see [token-prices.md](token-prices.md)).

---

## Philidor (DeFi Vault Risk Scoring — No API Key, Hosted)

Scores 700+ DeFi vaults across Morpho, Aave, Spark, Yearn, Beefy, Compound, and Uniswap. Three-vector risk framework (Asset 40%, Platform 40%, Governance 20%) with Prime/Core/Edge tier classification. Use after identifying yield opportunities via DefiLlama to assess risk before bridging funds.

### Installation

Hosted endpoint (no install, no key):

```bash
claude mcp add --transport http philidor https://mcp.philidor.io/api/mcp
```

### Key Tools

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `search_vaults` | `query`, `chain`, `protocol`, `asset`, `riskTier`, `minTvl`, `sortBy`, `sortOrder`, `limit` | Search and filter DeFi vaults by chain, protocol, asset, risk tier, TVL |
| `get_vault` | `id`, `network`, `address` | Detailed vault info including risk breakdown and historical snapshots |
| `get_vault_risk_breakdown` | `network`\*, `address`\* | Detailed risk vectors: Asset Composition, Platform Code, Governance scores |
| `compare_vaults` | `vaults`\* (array of 2-3) | Side-by-side comparison on TVL, APR, risk score, audit status |
| `find_safest_vaults` | `asset`, `chain`, `minTvl` | Top 10 safest vaults, filtered by asset/chain/TVL |
| `get_protocol_info` | `protocolId`\* | Protocol details: TVL, vault count, auditors, security incidents |
| `get_curator_info` | `curatorId`\* | Curator's managed vaults, TVL, chain distribution |
| `get_market_overview` | (none) | DeFi vault market: total TVL, vault count, risk distribution by protocol |
| `explain_risk_score` | `score`\* (number) | Explain a risk score: tier, calculation method, thresholds |
| `list_vaults_with_incidents` | (none) | Vaults with critical incidents in the last 365 days |

### Example: Assess Vault Risk After Bridging

```
1. Find yield via DefiLlama → get_pools → identify high-APY vault on Arbitrum
2. Check risk → get_vault_risk_breakdown (network: "arbitrum", address: "0x...")
3. Compare → find_safest_vaults (asset: "USDC", chain: "arbitrum")
4. If acceptable → bridge via ../swap/SKILL.md. If high → suggest safer vault.
```

---

## Arcadia Finance (LP Management & Lending — No API Key, Hosted)

DeFi LP management and lending protocol with tools for pool analytics, LP strategy evaluation, and position management. Use to assess lending pool rates and LP strategies on destination chains before/after bridging.

### Installation

Hosted endpoint (no install, no key):

```bash
claude mcp add --transport http arcadia https://mcp.arcadia.finance/mcp
```

### Key Tools (Read — Analytics)

| Tool | Parameters (\* = required) | Description |
|------|-----------|-------------|
| `read_pool_list` | `chain_id` | All lending pools: TVL, utilization, liquidity, interest rate |
| `read_pool_info` | `pool_address`\*, `days`, `chain_id` | Pool detail with APY history over time |
| `read_strategy_list` | `featured_only`, `limit`, `offset`, `chain_id` | LP strategies with APY per range width |
| `read_strategy_info` | `strategy_id`\*, `chain_id` | Full strategy detail: APY, range widths |
| `read_asset_list` | `search`, `chain_id` | Supported collateral assets |
| `read_asset_prices` | `asset_addresses`\*, `chain_id` | USD prices for assets |
| `read_account_info` | `account_address`\*, `chain_id` | Account health factor, collateral, debt |
| `read_account_pnl` | `account_address`\*, `chain_id` | PnL and yield earned |
| `read_strategy_recommendation` | `account_address`\*, `chain_id` | Rebalancing recommendation for an account |

### Key Tools (Write — Position Management)

Tools for building transactions: `write_account_deposit`, `write_account_withdraw`, `write_account_borrow`, `write_account_add_liquidity` (flash-action: deposit + LP in one tx), `write_account_swap` (flash-action: swap within account). All require `account_address`\* and `chain_id`.

### Example: Evaluate LP Strategies Before Bridging

```
1. List featured strategies → read_strategy_list (featured_only: true, chain_id: 8453)
2. Get detail → read_strategy_info (strategy_id from step 1)
3. Check lending rates → read_pool_list (chain_id: 8453)
4. Compare APY vs risk → advise on best deployment after bridging.
```
