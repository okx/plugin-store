---
name: axon-xlayer-intelligence
description: "X Layer DeFi intelligence skill — query gas, blocks, Uniswap V3 pools, wallet portfolios, swap quotes, yield opportunities, and arbitrage signals on X Layer (Chain ID 196) via 17 MCP tools"
version: "1.0.0"
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
---

# AXON — X Layer DeFi Intelligence

## Overview

AXON gives AI agents real-time onchain intelligence for **X Layer (Chain ID 196)** — OKX's zkEVM Layer 2 powered by OKB. It exposes 17 production-ready MCP tools covering gas analytics, block monitoring, Uniswap V3 pool data, wallet portfolio analysis (with AI risk scoring), DEX swap routing via OKX aggregator, yield farming discovery, and arbitrage signals. A natural language `/api/chat` endpoint lets agents skip tool selection and ask questions in plain English. Premium tools are gated behind an **x402 micro-payment** (OKB on X Layer) that is verified on-chain before execution.

**Live API:** `https://axon-onld.onrender.com`  
**Live Dashboard:** `https://axon-six-amber.vercel.app`  
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
3. No API key is required for free tools. All 17 tools are publicly callable.

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

AXON uses Groq LLaMA 3.3 70B to route intent to the correct tool and format the response as a natural language answer. Supported intents: gas, blocks, yield, pools, arbitrage, wallet analysis, swap quotes, market overview.

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
