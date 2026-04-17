---
name: bobby-protocol
description: "Adversarial AI trading intelligence on X Layer. 3-agent debate (Alpha Hunter vs Red Team vs CIO), Judge Mode (6 dimensions), adversarial bounties, conviction oracle. 15 MCP tools — 10 free, 5 premium via x402. Use when agents need: market analysis, trading signals, security scans, debate-quality intelligence, or want to challenge AI trading decisions on-chain. Triggers: 'bobby analyze', 'bobby debate', 'bobby intel', 'bobby signals', 'bobby judge', 'bobby bounty', 'trade analysis', 'conviction score', 'adversarial review'."
license: MIT
metadata:
  author: bobby-protocol
  version: "3.0.0"
  homepage: "https://bobbyprotocol.xyz"
---

# Bobby Protocol — Adversarial Trading Intelligence

3-agent debate system for crypto trading on OKX X Layer. Alpha Hunter proposes, Red Team attacks, CIO decides. Judge Mode audits on 6 dimensions. Anyone can stake OKB to prove Bobby was wrong.

## Pre-flight Checks

Bobby is a hosted MCP server — no local installation required. Before calling:

1. Verify the endpoint is reachable:
   ```bash
   curl -s https://bobbyprotocol.xyz/api/mcp-http | head -1
   ```
   Expected: JSON with `name: "bobby-protocol"`.

2. For premium tools, ensure you have a wallet with OKB on X Layer (Chain 196, RPC: `https://xlayerrpc.okx.com/`).

3. No API keys needed for free tools. Premium tools use x402 payment (on-chain tx).

## Commands

### bobby_intel — Full Market Intelligence

**When to use:** Agent needs a comprehensive market briefing with regime, signals, and conviction.

```bash
curl -X POST https://bobbyprotocol.xyz/api/mcp-http \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"bobby_intel","arguments":{}},"id":"1"}'
```

**Output:** 10-source intelligence briefing with market regime, whale signals, technical confluence, fear/greed index, and conviction score.

### bobby_ta — Technical Analysis

**When to use:** Agent needs SMA, RSI, MACD, Bollinger, S/R levels for a specific asset.

```bash
curl -X POST https://bobbyprotocol.xyz/api/mcp-http \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"bobby_ta","arguments":{"symbol":"BTC"}},"id":"2"}'
```

**Output:** Technical indicator matrix with composite score and bias label.

### bobby_stats — Track Record

**When to use:** Agent wants to verify Bobby's credibility before paying for analysis.

```bash
curl -X POST https://bobbyprotocol.xyz/api/mcp-http \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"bobby_stats","arguments":{}},"id":"3"}'
```

**Output:** Win rate, total trades, PnL, equity curve, and recent decisions.

### bobby_analyze — Full Analysis (PREMIUM, 0.001 OKB)

**When to use:** Agent needs Bobby's full 3-agent debate on a specific asset.

**Step 1:** Call without payment to get challenge:
```bash
curl -X POST https://bobbyprotocol.xyz/api/mcp-http \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"bobby_analyze","arguments":{"symbol":"OKB"}},"id":"4"}'
```

**Step 2:** Pay on X Layer — call `payMCPCall(challengeId, "bobby_analyze")` with 0.001 OKB on contract `0xD9540D770C8aF67e9E6412C92D78E34bc11ED871`.

**Step 3:** Retry with payment proof:
```bash
curl -X POST https://bobbyprotocol.xyz/api/mcp-http \
  -H "Content-Type: application/json" \
  -H "x-402-payment: <txHash>" \
  -H "x-challenge-id: <challengeId>" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"bobby_analyze","arguments":{"symbol":"OKB"}},"id":"5"}'
```

**Output:** Alpha Hunter thesis, Red Team attack, CIO verdict with conviction score, on-chain proof link.

### bobby_debate — Full 3-Agent Debate (PREMIUM, 0.001 OKB)

**When to use:** Agent wants the complete adversarial debate transcript.

Same x402 flow as bobby_analyze. Returns full Alpha Hunter / Red Team / CIO debate with conviction scoring.

### bobby_judge — Independent Debate Audit (PREMIUM, 0.001 OKB)

**When to use:** Agent wants a 6-dimension audit of Bobby's latest debate.

Same x402 flow. Returns scores for DATA_INTEGRITY, ADVERSARIAL_QUALITY, DECISION_LOGIC, RISK_MANAGEMENT, CALIBRATION_ALIGNMENT, NOVELTY.

### bobby_security_scan — Token Security Scan (PREMIUM, 0.001 OKB)

**When to use:** Agent needs to check a token for honeypot, rug-pull, or contract risks.

Same x402 flow. Uses OKX OnchainOS Security API under the hood.

### bobby_wallet_portfolio — Multi-Chain Portfolio (PREMIUM, 0.001 OKB)

**When to use:** Agent needs a comprehensive portfolio audit across chains.

Same x402 flow. Returns holdings, PnL, risk exposure across Ethereum, X Layer, Solana, Base.

### bobby_bounty_list — List Adversarial Bounties

**When to use:** Agent wants to see open challenges against Bobby's decisions.

```bash
curl -X POST https://bobbyprotocol.xyz/api/mcp-http \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"bobby_bounty_list","arguments":{"limit":5}},"id":"6"}'
```

**Output:** Recent bounties with dimension, reward, status, expiry, and challenge count.

### bobby_bounty_post — Build Bounty Calldata

**When to use:** Agent wants to post a bounty challenging Bobby's debate quality.

Returns unsigned transaction calldata for BobbyAdversarialBounties.postBounty(). The agent signs and broadcasts.

### bobby_bounty_challenge — Build Challenge Calldata

**When to use:** Agent wants to submit evidence against an open bounty.

Returns unsigned transaction calldata for BobbyAdversarialBounties.submitChallenge().

### All Available Tools

**Free:** `bobby_intel`, `bobby_ta`, `bobby_stats`, `bobby_xlayer_signals`, `bobby_xlayer_quote`, `bobby_wallet_balance`, `bobby_dex_trending`, `bobby_dex_signals`, `bobby_bounty_list`, `bobby_bounty_get`

**Premium (0.001 OKB):** `bobby_analyze`, `bobby_debate`, `bobby_judge`, `bobby_security_scan`, `bobby_wallet_portfolio`

**Bounty builders:** `bobby_bounty_post`, `bobby_bounty_challenge`

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `402 Payment Required` | Premium tool called without payment | Follow x402 flow: get challengeId, pay on-chain, retry with headers |
| `challenge expired` | Challenge older than 15 minutes | Request a new challenge by calling the tool again without payment |
| `challenge already consumed` | Replay attempt | Each challengeId is single-use; request a new one |
| `Insufficient MCP fee` | Payment below 0.001 OKB | Send exactly 0.001 OKB (1000000000000000 wei) |
| `500 Internal Server Error` | Temporary API issue | Retry after 5 seconds |

## Security

- Bobby never custodies user funds. Bounty tools return unsigned calldata only.
- Premium payments go to AgentEconomyV2 contract on X Layer, not to Bobby's wallet.
- All on-chain proofs are publicly verifiable on OKLink.
- x402 challenges expire after 15 minutes and are single-use (anti-replay).
- Token security scans use OKX OnchainOS Security API — treat results as advisory, not guarantees.

## OnchainOS APIs Used

- Token risk scanning (`/api/v5/dex/security/token-scan`)
- Smart money signals (`/api/v6/dex/market/signal/list`)
- DEX aggregator quotes (`/api/v6/dex/aggregator/quote`)
- Technical indicators (`/api/v5/aigc/mcp/indicators`)
- Market tickers, candles, funding rates, open interest
- Address analysis and wallet PnL (`/api/v5/dex/market/wallet-pnl`)
- x402 payment settlement on X Layer

## Contracts on X Layer (196)

- AgentEconomyV2: `0xD9540D770C8aF67e9E6412C92D78E34bc11ED871`
- ConvictionOracle: `0x03FA39B3a5B316B7cAcDabD3442577EE32Ab5f3A`
- TrackRecord: `0xF841b428E6d743187D7BE2242eccC1078fdE2395`
- AdversarialBounties: `0xa8005ab465a0e02cb14824cd0e7630391fba673d` (verified on OKLink)
