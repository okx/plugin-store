---
title: On-Chain Explorer — Address, Transaction, Contract, Portfolio
impact: HIGH
impactDescription: "Verify transactions and inspect addresses after bridge/swap operations"
tags: blockscout, hive, explorer, transactions, address, contract, abi, ens, portfolio
---

# On-Chain Explorer

Look up addresses, transactions, contracts, and token transfers on-chain. Primary use after deBridge operations: verify the transaction landed, inspect contract ABIs, or check destination wallet state.

## Blockscout MCP (Primary — No API Key, 3000+ Chains)

Official hosted MCP covering 3,000+ EVM-compatible chains. No installation or API key required.

### Installation

Hosted endpoint (recommended):

```bash
claude mcp add --transport http blockscout https://mcp.blockscout.com/mcp
```

### Key Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `get_chains_list` | — | All known blockchain networks |
| `get_block_info` | `chain_id`, `number_or_hash`, `include_transactions` | Block details by number or hash |
| `get_address_info` | `chain_id`, `address` | Balance, ENS name, contract status |
| `get_tokens_by_address` | `chain_id`, `address`, `cursor` | ERC-20 holdings with market data |
| `get_transactions_by_address` | `chain_id`, `address`, `age_from` (required), `age_to`, `methods`, `cursor` | Transactions in a time range |
| `get_token_transfers_by_address` | `chain_id`, `address`, `age_from` (required), `age_to`, `token`, `cursor` | Token transfers by address and timeframe |
| `get_transaction_info` | `chain_id`, `transaction_hash`, `include_raw_input` | Full transaction details with decoded input |
| `get_contract_abi` | `chain_id`, `address` | Smart contract ABI (verified contracts) |
| `inspect_contract_code` | `chain_id`, `address`, `file_name` | Verified contract source code |
| `lookup_token_by_symbol` | `chain_id`, `symbol` | Find token by symbol or name |
| `get_address_by_ens_name` | `name` | ENS to Ethereum address resolution |
| `get_block_number` | `chain_id`, `datetime` | Block number at a specific time |
| `nft_tokens_by_address` | `chain_id`, `address`, `cursor` | NFTs owned by address |
| `read_contract` | `chain_id`, `address`, `abi`, `function_name`, `args`, `block` | Read-only contract call |
| `direct_api_call` | `chain_id`, `endpoint_path`, `query_params`, `cursor` | Raw Blockscout API access |

### Chain IDs

Blockscout uses standard EVM chain IDs. Common ones for deBridge:

| Chain | Chain ID |
|-------|----------|
| Ethereum | 1 |
| Arbitrum | 42161 |
| Polygon | 137 |
| BSC | 56 |
| Base | 8453 |
| Optimism | 10 |
| Avalanche | 43114 |
| Linea | 59144 |

### Example: Verify Bridge Transaction on Destination Chain

```
1. After deBridge bridge completes, take the destination tx hash.

2. Call mcp__blockscout__get_transaction_info:
     chain_id: "42161"                    // Arbitrum
     transaction_hash: "0xabc123..."
     include_raw_input: false

3. Confirm status is "ok" and the expected token transfer is present.

4. Call mcp__blockscout__get_tokens_by_address:
     chain_id: "42161"
     address: "<destination-wallet>"

5. Verify the expected token balance increased.
```

### Example: Look Up Token by Symbol on a Chain

```
Call mcp__blockscout__lookup_token_by_symbol:
  chain_id: "8453"        // Base
  symbol: "USDC"

Returns: contract address, name, decimals, total supply.
```

---

## Hive Intelligence (Multi-Chain Analytics — No API Key, Hosted)

300+ tools across 9 sub-servers covering market data, DeFi, DEX, portfolios, tokens, NFTs, sentiment, network infra, and search. Each sub-server exposes tools directly — no meta-tool indirection needed.

### Installation

Connect to individual sub-servers by category. Each is a separate hosted endpoint:

```bash
# Add the sub-servers you need
claude mcp add --transport http hive-portfolio https://mcp.hiveintelligence.xyz/hive_portfolio_wallet/mcp
claude mcp add --transport http hive-tokens https://mcp.hiveintelligence.xyz/hive_token_contract/mcp
claude mcp add --transport http hive-defi https://mcp.hiveintelligence.xyz/hive_defi_protocol/mcp
claude mcp add --transport http hive-dex https://mcp.hiveintelligence.xyz/hive_onchain_dex/mcp
claude mcp add --transport http hive-market https://mcp.hiveintelligence.xyz/hive_market_data/mcp
```

### Sub-Server Catalog

| Sub-Server | Endpoint | Tools | Focus |
|------------|----------|-------|-------|
| Portfolio & Wallet | `mcp.hiveintelligence.xyz/hive_portfolio_wallet/mcp` | 38 | Wallet balances, token holdings, DeFi positions, transaction history across chains |
| Token & Contract | `mcp.hiveintelligence.xyz/hive_token_contract/mcp` | 27 | Token info, top holders, holder charts, token filtering, contract details |
| DeFi Protocol | `mcp.hiveintelligence.xyz/hive_defi_protocol/mcp` | 23 | Protocol TVL, fees, yields, global DeFi stats |
| On-Chain DEX | `mcp.hiveintelligence.xyz/hive_onchain_dex/mcp` | 44 | Pool analytics, trending pools, OHLCV, trades, DEX data |
| Market Data | `mcp.hiveintelligence.xyz/hive_market_data/mcp` | 80 | Prices, market charts, OHLCV, gainers/losers, exchange data |
| NFT Analytics | `mcp.hiveintelligence.xyz/hive_nft_analytics/mcp` | 37 | NFT collections, marketplace data, floor prices, tickers |
| Social Sentiment | `mcp.hiveintelligence.xyz/hive_social_sentiment/mcp` | 17 | Topic news/posts, sentiment metrics, trending topics |
| Network Infra | `mcp.hiveintelligence.xyz/hive_network_infrastructure/mcp` | 24 | Network status, gas prices, blockchain stats |
| Search & Discovery | `mcp.hiveintelligence.xyz/hive_search_discovery/mcp` | 10 | Search tokens/protocols, trending, categories, new coins |

### Key Tools by Use Case

**Portfolio (hive_portfolio_wallet):**

| Tool | Parameters (* = required) | Description |
|------|-----------|-------------|
| `get_wallet_balance` | `id`*, `chain_id` | Wallet balance on a chain |
| `get_wallet_token_balances` | `id`*, `is_all`, `chain_ids` | Token balances across chains |
| `get_wallet_defi_positions_all_chains` | `id`*, `chain_ids` | DeFi positions across all chains |
| `get_wallet_history` | `id`*, `chain_id`* | Transaction history |

**Token Data (hive_token_contract):**

| Tool | Parameters (* = required) | Description |
|------|-----------|-------------|
| `get_token_info` | `network`*, `address`* | Token metadata and on-chain info |
| `get_token_top_holders` | `network`*, `address`* | Top token holders |
| `get_token_details` | `networkId`*, `address`* | Detailed token information |

### Example: Portfolio Overview Before Bridge

```
1. Call mcp__hive_portfolio__get_wallet_token_balances:
     id: "0xYourWallet"
     is_all: true

2. Identify which chains have the most of the target token.

3. Bridge from the chain with the highest balance.
```

---

## When to Use Which

| Scenario | MCP | Tool |
|----------|-----|------|
| Verify a single transaction | Blockscout | `get_transaction_info` |
| Look up contract ABI or source | Blockscout | `get_contract_abi` / `inspect_contract_code` |
| Read contract state | Blockscout | `read_contract` |
| ENS resolution | Blockscout | `get_address_by_ens_name` |
| Token holdings for an address | Blockscout | `get_tokens_by_address` |
| Transfer history by time range | Blockscout | `get_transactions_by_address` |
| Block details | Blockscout | `get_block_info` |
| Multi-chain portfolio overview | Hive (hive-portfolio) | `get_wallet_token_balances` |
| Social sentiment analysis | Hive (hive-sentiment) | Connect `hive_social_sentiment` sub-server |
| NFT analytics | Hive (hive-nft) | Connect `hive_nft_analytics` sub-server |
