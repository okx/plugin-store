---
name: axon-xlayer-intelligence
description: "AXON — X Layer neural intelligence layer. 43 MCP tools: portfolio, 6-source token security, OKLink explorer, NFT, yield products, Uniswap V3 analytics, cross-chain bridge, address/URL risk check, smart money signals, and x402 payment gates on X Layer (Chain ID 196)"
version: "2.0.0"
author: "MUTHUKUMARAN K"
tags:
  - xlayer
  - okb
  - uniswap
  - defi
  - onchainos
  - mcp
  - yield
  - portfolio
  - x402
  - security
  - nft
  - explorer
  - oklink
  - bridge
  - intelligence
---

# AXON — X Layer Neural Intelligence Layer

## Quick Install

**One command — any AI agent, anywhere:**

```bash
# Claude Code
claude mcp add axon-xlayer --transport http https://axon-onld.onrender.com

# OKX Plugin Store
npx skills add okx/plugin-store --skill axon-xlayer-intelligence
```

After install, all 43 tools are immediately available. No API key required for free tools.

---

## Overview

AXON gives AI agents real-time onchain intelligence for **X Layer (Chain ID 196)** — OKX's zkEVM Layer 2 powered by OKB. It exposes **43 production-ready MCP tools** covering:

- **Portfolio intelligence** — token balances, net worth, DeFi positions, NFT holdings, TX history
- **Token security** — 6-source scanner (OKX Security API, Onchain OS, DexScreener, DefiLlama, Uniswap V3, OKLink) with honeypot, rug, and holder concentration analysis
- **Address & URL risk** — blacklist check, phishing URL detection via OKX Onchain OS
- **OKLink Explorer** — address info, block detail, contract verification, pending TXs, rich list, internal traces, gas estimation
- **Uniswap V3** — top pools, pool OHLC, fee revenue, protocol stats, smart money velocity signals
- **Swap & bridge** — best route via OKX DEX aggregator, cross-chain bridge quotes, swap calldata
- **Yield products** — farming opportunities on X Layer with APY data
- **x402 premium gate** — premium tools gated by OKB micro-payment, verified on-chain via OKLink

A natural language `/api/chat` endpoint lets agents skip tool selection and ask questions in plain English via Groq LLaMA 3.3 70B.

**Live API:** `https://axon-onld.onrender.com`  
**Live Dashboard:** `https://axon-six-amber.vercel.app`  
**API Docs:** `https://axon-onld.onrender.com/docs`  
**MCP Tools:** `https://axon-onld.onrender.com/mcp/tools`  
**Chain:** X Layer Mainnet — Chain ID 196, native token OKB  
**Agentic Wallet:** `0xDb82c0d91E057E05600C8F8dc836bEb41da6df14`

---

## Pre-flight Checks

Before using this skill, verify:

1. The AXON API is reachable:
   ```bash
   curl https://axon-onld.onrender.com/health
   # Expected: {"status":"ok","service":"AXON"}
   ```
2. For premium tools (`analyze_wallet`, `compare_wallets`, `find_arbitrage_opportunities`), you need OKB on X Layer to pay via x402. Check pricing:
   ```bash
   curl https://axon-onld.onrender.com/api/x402/pricing
   ```
3. No API key is required for free tools. All 43 tools are publicly callable.

> **Note:** The Render backend may cold-start in ~30s if unused. Retry once if you get a timeout.

---

## Commands

All tools are called via `POST https://axon-onld.onrender.com/mcp/call`.

### get_gas_price — X Layer Gas Analytics

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_gas_price", "arguments": {}}'
```

**When to use:** When the user asks "what's the gas price?", "is it cheap to transact?", or "when should I execute my swap?"  
**Output:** `gas_price_gwei`, `max_priority_fee_gwei`, `estimated_transfer_cost_usd`  
**Example response:**
```json
{
  "result": {
    "success": true,
    "chain": "X Layer",
    "gas_price_gwei": 0.021,
    "max_priority_fee_gwei": 0.001,
    "estimated_transfer_cost_usd": 0.0000222
  }
}
```

---

### get_block_info — Latest Block Data

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_block_info", "arguments": {"block": "latest"}}'
```

**When to use:** When the user asks about network health, block time, or congestion.  
**Output:** `block_number`, `tx_count`, `gas_utilization_pct`, `base_fee_gwei`

---

### get_market_overview — X Layer DeFi Snapshot

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_market_overview", "arguments": {}}'
```

**When to use:** When the user asks "what's happening on X Layer?" or wants a market summary.  
**Output:** Combined gas, block height, top Uniswap pools, and gas cost estimate.

---

### get_uniswap_top_pools — Uniswap V3 Pool Rankings

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_uniswap_top_pools", "arguments": {"limit": 5}}'
```

**When to use:** When the user asks about top liquidity pools, TVL rankings, or where to provide liquidity on X Layer.  
**Output:** Pool pairs, TVL USD, volume 24h, fee tier, fee APY estimate.  
**Parameters:**
- `limit` (int, optional) — max pools to return (default 5, max 20)

---

### get_token_price — Token Price (OKX Onchain OS)

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_token_price", "arguments": {"token_address": "0x1e4a5963abfd975d8c9021ce480b42188849d41d", "chain_id": "196"}}'
```

**When to use:** When the user asks the price of a specific token on X Layer.  
**Parameters:**
- `token_address` (string, required) — ERC-20 contract address on X Layer
- `chain_id` (string, optional) — defaults to "196" (X Layer)

---

### get_swap_quote — DEX Swap Quote via OKX Aggregator

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "tool_name": "get_swap_quote",
    "arguments": {
      "from_token": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      "to_token": "0x1e4a5963abfd975d8c9021ce480b42188849d41d",
      "amount": "1000000000000000000",
      "chain_id": "196"
    }
  }'
```

**When to use:** When the user asks "how much USDC will I get for 1 OKB?" or wants the best swap route.  
**Output:** Expected output amount, price impact, route path, estimated gas.  
**Parameters:**
- `from_token` — use `0xeeee...eeee` for native OKB
- `to_token` — ERC-20 address of the destination token
- `amount` — amount in wei (1 OKB = 1e18)
- `chain_id` — "196" for X Layer

---

### get_yield_opportunities — Yield Farming Scanner

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_yield_opportunities", "arguments": {"min_apy": 5.0}}'
```

**When to use:** When the user asks "where can I earn yield?", "best APY on X Layer?", or "where should I LP?"  
**Output:** Pool opportunities ranked by estimated fee APY, with TVL, volume, and risk level.  
**Parameters:**
- `min_apy` (float, optional) — filter to pools above this APY percentage (default 0)

---

### get_wallet_portfolio — Portfolio Analysis

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "tool_name": "get_wallet_portfolio",
    "arguments": {"address": "0xDb82c0d91E057E05600C8F8dc836bEb41da6df14"}
  }'
```

**When to use:** When the user asks what tokens a wallet holds on X Layer.  
**Output:** Token holdings, balances, USD values (using OKX Onchain OS Wallet API).  
**Parameters:**
- `address` (string, required) — EVM address to analyze

---

### analyze_wallet 🔒 — AI Portfolio Analysis (Premium)

> **x402 Required:** Send `X-PAYMENT: 0xYourTxHash` header (0.001 OKB on X Layer to `0xDb82c0d91E057E05600C8F8dc836bEb41da6df14`)

```bash
# First verify your payment will be accepted
curl -X POST https://axon-onld.onrender.com/api/x402/verify \
  -H "Content-Type: application/json" \
  -d '{"tx_hash": "0xYourTxHash", "tool_name": "analyze_wallet"}'

# Then call the premium tool
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -H "X-PAYMENT: 0xYourTxHash" \
  -d '{
    "tool_name": "analyze_wallet",
    "arguments": {
      "address": "0xDb82c0d91E057E05600C8F8dc836bEb41da6df14",
      "include_ai_insights": true
    }
  }'
```

**When to use:** When the user wants a comprehensive AI-generated risk analysis of an X Layer wallet.  
**Output:** Risk score (0–100), DeFi positions, top holdings, AI narrative, recommendations.  
**Parameters:**
- `address` (string, required) — EVM address to analyze
- `include_ai_insights` (bool, optional) — include LLM-generated narrative (default true)

---

### find_arbitrage_opportunities 🔒 — Arbitrage Scanner (Premium)

> **x402 Required:** 0.001 OKB

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -H "X-PAYMENT: 0xYourTxHash" \
  -d '{
    "tool_name": "find_arbitrage_opportunities",
    "arguments": {
      "token_address": "0x1e4a5963abfd975d8c9021ce480b42188849d41d",
      "amount_usd": 1000
    }
  }'
```

**When to use:** When the user asks about price discrepancies, MEV opportunities, or arbitrage on X Layer.  
**Output:** Arbitrage opportunities with expected profit, token path, and pool addresses.

---

### get_native_balance — OKB Balance Lookup

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_native_balance", "arguments": {"address": "0xDb82c0d91E057E05600C8F8dc836bEb41da6df14"}}'
```

**When to use:** When the user asks "how much OKB does this wallet have?" or "check native balance."  
**Output:** OKB balance in native units and wei.

---

### get_xlayer_stats — Chain Metadata

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_xlayer_stats", "arguments": {}}'
```

**When to use:** When the user asks about X Layer (chain ID, RPC, bridges, ecosystem).  
**Output:** Chain ID, native token, RPC URL, explorer, bridge URL, ecosystem protocols.

---

### scan_token_security — 6-Source Token Security Scanner

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "tool_name": "scan_token_security",
    "arguments": {"token_address": "0x1e4a5963abfd975d8c9021ce480b42188849d41d"}
  }'
```

**When to use:** When the user asks "is this token safe?", "is it a honeypot?", "should I buy X token?", "is this a rug pull?", or any security/scam/risk question about a token address.  
**Output:** Composite risk score (0–100, higher = riskier), verdict (SAFE / CAUTION / RISKY / DANGEROUS), 6 independent source scores, honeypot flag, holder concentration, top holder percentage, pair age in days, liquidity USD, volume/TVL ratio, and a list of all risk flags raised.  
**Parameters:**
- `token_address` (string, required) — ERC-20 contract address on X Layer

**Risk score tiers:**

| Score | Verdict | Meaning |
|-------|---------|---------|
| 0–19 | SAFE | Low risk, passes all checks |
| 20–44 | CAUTION | Minor flags — review before investing |
| 45–64 | RISKY | Multiple red flags — high caution |
| 65–100 | DANGEROUS | Likely honeypot or rug — do not buy |

**6 sources queried in parallel:**
1. OKX DEX Security API (`/dex/security/token`)
2. OKX Onchain OS Advanced (`/wallet/token/security-info`)
3. DexScreener (pair age, volume, liquidity, FDV)
4. DefiLlama Yields (APY sanity check)
5. Uniswap V3 subgraph (holder concentration, top holder %)
6. OKLink (on-chain verification)

**Example response:**
```json
{
  "result": {
    "token_address": "0x1e4a5963...",
    "risk_score": 12,
    "verdict": "SAFE",
    "honeypot": false,
    "holder_count": 4821,
    "top_holder_pct": 8.3,
    "pair_age_days": 142,
    "liquidity_usd": 2100000,
    "volume_tvl_ratio": 0.18,
    "flags": [],
    "sources": {
      "okx_dex": 10,
      "okx_onchain": 8,
      "dexscreener": 15,
      "defillama": 5,
      "uniswap_v3": 12,
      "oklink": 10
    }
  }
}
```

---

### get_smart_money_signals — Smart Money Velocity Signals

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_smart_money_signals", "arguments": {"limit": 10}}'
```

**When to use:** When the user asks "what are whales buying?", "any hot tokens?", "smart money signals?", "what's being accumulated?", or wants to find high-conviction opportunities on X Layer.  
**Output:** Ranked list of Uniswap V3 pools with elevated volume/TVL ratios indicating potential smart money accumulation. Includes signal strength (STRONG / MODERATE), pair symbol, TVL, 24h volume, volume/TVL ratio, and fee tier.  
**Parameters:**
- `limit` (int, optional) — max signals to return (default 10, max 50)

**Signal strength thresholds:**

| Volume/TVL Ratio | Signal | Interpretation |
|-----------------|--------|---------------|
| ≥ 0.5 | STRONG | Very high activity relative to liquidity — smart money accumulation likely |
| 0.2–0.49 | MODERATE | Elevated activity — worth watching |
| < 0.2 | — | Normal activity, not returned |

**Example response:**
```json
{
  "result": {
    "signals": [
      {
        "pool": "WBTC/USDC",
        "signal": "STRONG",
        "volume_tvl_ratio": 0.74,
        "tvl_usd": 1200000,
        "volume_24h_usd": 888000,
        "fee_tier": "0.05%",
        "pool_address": "0xabc..."
      }
    ],
    "scanned_pools": 47,
    "timestamp": "2026-04-11T12:00:00Z"
  }
}
```

---

### get_wallet_net_worth — Total Portfolio Value Across All Chains

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_wallet_net_worth", "arguments": {"address": "0xDb82c0d91E057E05600C8F8dc836bEb41da6df14"}}'
```

**When to use:** When the user wants total USD value across all chains, not just X Layer.  
**Output:** `total_usd`, per-chain breakdown.

---

### get_token_detail — Rich Token Metadata

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_token_detail", "arguments": {"token_address": "0x1e4a5963abfd975d8c9021ce480b42188849d41d", "chain_id": "196"}}'
```

**When to use:** When the user wants FDV, holder count, social links, or a description of a token.  
**Output:** `name`, `symbol`, `decimals`, `holder_count`, `fdv_usd`, `description`, `website`, `twitter`.

---

### lookup_transaction — Decode Any Transaction

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "lookup_transaction", "arguments": {"tx_hash": "0x14a9bd9d2cbbb80be3373dd8b414104d107466247c48a2bd3c8ceb8eee58360b"}}'
```

**When to use:** When the user wants to decode a TX hash — status, from/to, value, method called.  
**Output:** `status`, `from`, `to`, `value`, `method`, `block`, `timestamp`, `gas_used`.

---

### check_address_security — Address Risk Check

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "check_address_security", "arguments": {"address": "0xSomeAddress"}}'
```

**When to use:** When the user wants to verify if a wallet or contract is blacklisted, flagged, or associated with phishing/scams.  
**Output:** `is_blacklisted`, `risk_level`, `address_type`, `labels`, `is_contract`.

---

### check_url_safety — Phishing URL Detection

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "check_url_safety", "arguments": {"url": "https://suspicious-site.xyz"}}'
```

**When to use:** When the user wants to verify a website URL before connecting their wallet or clicking a link.  
**Output:** `is_malicious`, `risk_level`, `category`, `description`.

---

### get_nft_holdings — NFT Portfolio

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_nft_holdings", "arguments": {"address": "0xDb82c0d91E057E05600C8F8dc836bEb41da6df14"}}'
```

**When to use:** When the user asks what NFTs a wallet holds on X Layer.  
**Output:** `nft_count`, array of NFTs with `name`, `collection_name`, `token_id`, `image_url`.

---

### get_yield_products — Available Yield Products

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_yield_products", "arguments": {}}'
```

**When to use:** When the user asks about staking, farming, or structured yield products available on X Layer.  
**Output:** Array of products with `name`, `protocol`, `asset`, `apy`, `tvl`.

---

### get_cross_chain_quote — Cross-Chain Bridge Quote

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{
    "tool_name": "get_cross_chain_quote",
    "arguments": {
      "from_chain_id": "1",
      "to_chain_id": "196",
      "from_token": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      "to_token": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      "amount": "1000000000000000000",
      "user_wallet": "0xYourWallet"
    }
  }'
```

**When to use:** When the user wants to bridge assets to/from X Layer.  
**Output:** `estimated_output`, `bridge_fee`, `estimated_time_seconds`, `route`.

---

### get_address_info — OKLink Address Info

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_address_info", "arguments": {"address": "0xDb82c0d91E057E05600C8F8dc836bEb41da6df14"}}'
```

**When to use:** When the user asks about an address entity, OKB balance, or TX history on X Layer via OKLink.  
**Output:** `balance_okb`, `tx_count`, `first_tx_time`, `last_tx_time`, `is_contract`, `entity_tag`.

---

### get_block_detail — OKLink Block Detail

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_block_detail", "arguments": {"block_number": "57163818"}}'
```

**When to use:** When the user asks about a specific block: validator, gas used, base fee, TX count.  
**Output:** `number`, `hash`, `timestamp`, `tx_count`, `validator`, `gas_used`, `gas_limit`, `gas_utilization_pct`, `base_fee`.

---

### get_contract_info — OKLink Contract Verification

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_contract_info", "arguments": {"contract_address": "0x1e4a5963abfd975d8c9021ce480b42188849d41d"}}'
```

**When to use:** When the user wants to know if a contract is verified, who deployed it, and the deploy TX.  
**Output:** `is_verified`, `contract_name`, `compiler_version`, `creator`, `deploy_tx`, `deploy_time`, `license`.

---

### get_rich_list — Top Token Holders

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_rich_list", "arguments": {"limit": 10}}'
```

**When to use:** When the user asks "who holds the most OKB?" or wants top holders for any token.  
**Parameters:**
- `token_contract` (string, optional) — ERC-20 address; omit for native OKB
- `limit` (int, optional) — max 50

**Output:** Ranked list with `address`, `balance`, `pct_of_supply`, `is_contract`.

---

### get_internal_transactions — Internal Calls / Traces

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_internal_transactions", "arguments": {"tx_hash": "0xSomeTxHash"}}'
```

**When to use:** When the user wants to see internal contract calls within a complex transaction (e.g., DeFi protocol interactions).  
**Output:** `internal_call_count`, array of calls with `from`, `to`, `value`, `type`, `input`.

---

### get_uniswap_protocol_stats — Protocol-Level Stats

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_uniswap_protocol_stats", "arguments": {}}'
```

**When to use:** When the user asks about Uniswap V3's total TVL, all-time volume, or total pools on X Layer.  
**Output:** `total_tvl_usd`, `total_volume_usd`, `total_fees_usd`, `pool_count`, `tx_count`.

---

### get_pool_fees — Pool Fee Revenue & APY

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "get_pool_fees", "arguments": {"pool_address": "0xPoolAddress"}}'
```

**When to use:** When the user wants estimated fee APY and revenue for a specific Uniswap V3 pool.  
**Output:** `fee_apy_pct`, `fees_7d_usd`, `tvl_usd`, `volume_7d_usd`.

---

### search_pools_by_token — Find Pools for a Token

```bash
curl -X POST https://axon-onld.onrender.com/mcp/call \
  -H "Content-Type: application/json" \
  -d '{"tool_name": "search_pools_by_token", "arguments": {"token_address": "0x1e4a5963abfd975d8c9021ce480b42188849d41d", "limit": 5}}'
```

**When to use:** When the user wants to find all Uniswap V3 pools that include a specific token on X Layer.  
**Output:** Array of pools with `pair`, `tvl_usd`, `volume_24h`, `fee_tier`, `pool_address`.

---

## Natural Language Chat (Alternative to Direct Tool Calls)

Instead of selecting tools manually, agents can ask in plain English:

```bash
curl -X POST https://axon-onld.onrender.com/api/chat \
  -H "Content-Type: application/json" \
  -d '{"question": "What is the best yield farming opportunity on X Layer right now?"}'
```

**Response:**
```json
{
  "success": true,
  "tool_used": "get_yield_opportunities",
  "answer": "The highest yield on X Layer is the WBTC/WETH pool at 14.2% APY with $2.1M TVL...",
  "raw_data": { ... }
}
```

AXON uses Groq LLaMA 3.3 70B to route intent to the correct tool and format the response as a natural language answer. Supported intents: gas, blocks, yield, pools, arbitrage, wallet analysis, swap quotes, market overview, **token security scanning**, **smart money signals**.

---

## x402 Payment Flow

For premium tools, AXON performs full on-chain verification via OKLink before executing:

```
1. Send OKB to 0xDb82c0d91E057E05600C8F8dc836bEb41da6df14 on X Layer
2. Copy the transaction hash (0x...)
3. [Optional] POST /api/x402/verify {"tx_hash":"0x...", "tool_name":"analyze_wallet"}
   → {"valid": true, "verification": {"source": "oklink", ...}}
4. Call /mcp/call with X-PAYMENT: 0xYourTxHash header
5. AXON queries OKLink API to confirm tx on-chain, validates recipient + amount
6. Executes tool — tx marked as used (replay protection, 24h window)
```

**X-PAYMENT header formats accepted:**
- `X-PAYMENT: 0x<64-hex-chars>` — raw tx hash
- `X-PAYMENT: <base64("0x...")>` — base64 encoded
- `X-PAYMENT: <base64({"tx":"0x..."})>` — base64 JSON

---

## Agent Activity Feed

AXON runs an autonomous background agent that scans X Layer every 60 seconds:

```bash
curl https://axon-onld.onrender.com/api/agent/activity
```

Returns timestamped events: gas alerts (when < 0.05 gwei), yield signals (APY > 8%), block health checks, and action logs.

---

## Examples

### Example 1: Yield Opportunity Discovery

User asks: *"Where's the best place to earn yield on X Layer?"*

1. Call `get_yield_opportunities` with `{"min_apy": 8.0}`
2. Parse results — sort by `estimated_fee_apy_pct`
3. Present top pool: pair name, APY, TVL, fee tier
4. Suggest user add liquidity via `/api/chat` for natural language guidance

### Example 2: Wallet Intelligence Report

User asks: *"Analyze wallet 0xABC... and tell me if it's risky"*

1. Check if user has OKB payment ready (premium tool)
2. If not: call free `get_wallet_portfolio` first for basic holdings
3. If yes: call `analyze_wallet` with `X-PAYMENT` header for AI risk score
4. Return risk level (LOW/MEDIUM/HIGH), top positions, and AI recommendations

### Example 3: Finding the Best Swap

User asks: *"What's the best rate to swap 100 OKB to USDT on X Layer?"*

1. Get token addresses for OKB (`0xeeee...eeee`) and USDT on X Layer
2. Call `get_swap_quote` with `amount: "100000000000000000000"` (100 OKB in wei)
3. Return expected output, price impact, and best route

### Example 4: Gas Timing Decision

User asks: *"Is now a good time to do a large transaction?"*

1. Call `get_gas_price` — check `gas_price_gwei`
2. Call `get_block_info` — check `gas_utilization_pct`
3. If gas < 0.05 gwei AND utilization < 50%: "Yes, excellent conditions"
4. Otherwise: "Gas is elevated at X gwei, consider waiting"

### Example 5: Token Safety Check Before Buying

User asks: *"Is 0x1e4a5963... safe to buy?"*

1. Call `scan_token_security` with `{"token_address": "0x1e4a5963..."}`
2. Check `verdict` field — SAFE / CAUTION / RISKY / DANGEROUS
3. Check `honeypot` flag — if true, abort immediately
4. Check `flags` list — surface any specific risks to the user
5. Check `pair_age_days` and `liquidity_usd` — new pairs with low liquidity are higher risk
6. Return composite risk score and verdict with plain-English explanation

### Example 6: Finding Smart Money Accumulation

User asks: *"What are whales buying on X Layer right now?"*

1. Call `get_smart_money_signals` with `{"limit": 5}`
2. Filter to `signal: "STRONG"` entries (volume/TVL ≥ 0.5)
3. Present pool names, volume/TVL ratios, and TVL
4. Optionally cross-reference with `scan_token_security` for safety check on flagged pools

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `{"detail": "Not Found"}` | Wrong endpoint path | Check URL — use `/mcp/call` not `/mcp/tools` |
| HTTP 402 with `"error": "Payment Required"` | Calling premium tool without payment | Send OKB to agent wallet, include X-PAYMENT header |
| `"rejection_reason": "Payment already used"` | Tx hash reused | Each tx can only be used once — send a new payment |
| `"rejection_reason": "Wrong recipient"` | Paid wrong address | Send to `0xDb82c0d91E057E05600C8F8dc836bEb41da6df14` |
| `"rejection_reason": "Insufficient amount"` | Sent too little OKB | Check `/api/x402/pricing` for exact required amount |
| `"success": false, "error": "..."` | Tool-level error (RPC, API) | Retry once — X Layer RPC or OKX API may be temporarily slow |
| Connection timeout | Render cold-start | Wait 30s and retry — free tier spins down after inactivity |

---

## Security Notices

- **Read-only by default.** Free tools only read data from X Layer — no transaction signing.
- **Premium tools are also read-only** — they analyze data, not execute trades.
- **x402 payments are one-way.** Sending OKB to the agent wallet is irreversible. Verify the tool name and amount before sending.
- **No private keys handled.** AXON never touches or stores wallet private keys.
- **Risk level: LOW** for all analytics tools. Always consult a financial advisor before making investment decisions based on tool output.

---

## Skill Routing

- For executing actual swaps → use `okx-dex-swap` onchainos skill
- For CEX trading → use `okx-trade-mcp` skill
- For multi-chain portfolio data → use `okx-wallet-portfolio` skill
- For X Layer-specific onchain analytics → use this skill (axon-xlayer-intelligence)
- For natural language X Layer queries → use `/api/chat` endpoint directly
