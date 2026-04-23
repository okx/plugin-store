---
name: polymarketsignalbridge
description: "Cross-analyze Polymarket prediction odds with Hyperliquid funding rates and OKX smart-money signals to surface convergence and divergence opportunities"
version: "1.0.0"
author: "kin684660-commits"
tags:
  - polymarket
  - hyperliquid
  - signal
  - wallet-analysis
---

# Polymarket Signal Bridge

## Overview

This skill cross-analyzes three data sources to help users find trading opportunities:

1. **Polymarket** — prediction market probabilities and order book depth for real-world events
2. **Hyperliquid** — perpetual futures funding rates, open interest, and top-trader positions
3. **OKX OnchainOS signals** — smart-money wallet activity and on-chain leaderboard data

When Polymarket odds and Hyperliquid positioning **converge** (both pointing the same direction), it signals a high-conviction trade setup. When they **diverge**, it flags a potential mispricing worth investigating.

> **Risk Disclaimer**: This skill provides data aggregation and analysis only. It does not execute trades automatically. All trading decisions are made by the user. Prediction markets and perpetual futures involve substantial risk of loss. Only use funds you can afford to lose. This is not financial advice.

---

## Pre-flight Checks

Before using this skill, ensure the following are installed and configured:

1. Install the OKX OnchainOS CLI:
   ```bash
   npx skills add okx/onchainos-skills
   export PATH="$HOME/.local/bin:$PATH"
   ```

2. Verify installation:
   ```bash
   onchainos --help
   ```

3. No API keys are required for read-only Polymarket and Hyperliquid data queries.

4. Ensure internet connectivity to reach:
   - `clob.polymarket.com`
   - `gamma-api.polymarket.com`
   - `api.hyperliquid.xyz`

---

## Commands

### 1. Scan hot Polymarket markets

Fetch currently trending prediction markets with the highest volume and liquidity.

**When to use**: When the user asks "what's trending on Polymarket", "show me hot prediction markets", or wants to find markets worth analyzing.

```bash
curl -s "https://gamma-api.polymarket.com/markets?active=true&closed=false&order=volume&ascending=false&limit=10" \
  | python3 -c "
import json, sys
data = json.load(sys.stdin)
for m in data:
    print(f\"[{m.get('volume24hr','?')} vol] {m['question']} YES: {m.get('outcomePrices',['?'])[0]}\")
"
```

**Output**: List of top 10 markets by 24h volume, showing the question and current YES price (0-1 range, represents probability).

**Example output**:
```
[482310 vol] Will the Fed cut rates in May 2025? YES: 0.34
[301200 vol] Will BTC close above 100k this week? YES: 0.61
```

---

### 2. Get Polymarket order book for a specific market

Fetch live bid/ask depth for a given Polymarket token ID.

**When to use**: When the user wants to see liquidity depth or entry/exit conditions for a specific market.

```bash
curl -s "https://clob.polymarket.com/book?token_id=<TOKEN_ID>" \
  | python3 -c "
import json, sys
book = json.load(sys.stdin)
bids = book.get('bids', [])[:5]
asks = book.get('asks', [])[:5]
print('Top 5 bids:')
for b in bids: print(f\"  {b['price']} x {b['size']}\")
print('Top 5 asks:')
for a in asks: print(f\"  {a['price']} x {a['size']}\")
"
```

**Output**: Top 5 bid and ask levels with price and size.

---

### 3. Get Hyperliquid funding rate and open interest

Fetch the current funding rate and open interest for a perpetual contract on Hyperliquid.

**When to use**: When the user asks about funding rates or overall market positioning on Hyperliquid.

```bash
curl -s -X POST "https://api.hyperliquid.xyz/info" \
  -H "Content-Type: application/json" \
  -d '{"type": "metaAndAssetCtxs"}' \
  | python3 -c "
import json, sys
data = json.load(sys.stdin)
meta = data[0]['universe']
ctxs = data[1]
target = 'BTC'
for i, coin in enumerate(meta):
    if coin['name'] == target:
        ctx = ctxs[i]
        print(f'Symbol: {target}')
        print(f'Funding rate: {float(ctx[\"funding\"])*100:.4f}% per 8h')
        print(f'Open interest: {ctx[\"openInterest\"]} contracts')
        print(f'Mark price: {float(ctx[\"markPx\"]):.2f}')
        break
"
```

**Output**: Funding rate, open interest, and mark price. Positive funding means longs pay shorts. Negative funding means shorts pay longs.

---

### 4. Analyze a wallet address on Hyperliquid

Profile a wallet's positions and PnL on Hyperliquid.

**When to use**: When the user provides a wallet address and wants to know its positions and performance.

```bash
curl -s -X POST "https://api.hyperliquid.xyz/info" \
  -H "Content-Type: application/json" \
  -d '{"type": "clearinghouseState", "user": "<WALLET_ADDRESS>"}' \
  | python3 -c "
import json, sys
data = json.load(sys.stdin)
positions = data.get('assetPositions', [])
margin = data.get('crossMarginSummary', {})
print(f'Account value: {float(margin.get(\"accountValue\", 0)):.2f}')
print(f'Unrealized PnL: {float(margin.get(\"totalUnrealizedPnl\", 0)):.2f}')
print(f'Open positions: {len(positions)}')
for p in positions:
    pos = p.get('position', {})
    print(f'  {pos.get(\"coin\",\"?\")} size: {pos.get(\"szi\",\"?\")} entry: {float(pos.get(\"entryPx\",0)):.2f}')
"
```

**Output**: Total account value, unrealized PnL, and all open positions with entry price.

---

### 5. Fetch OKX smart-money signals

Pull top traders by PnL from OKX OnchainOS signal module.

**When to use**: When the user wants to see what wallets smart money is backing.

```bash
onchainos signal list --chain ethereum
```

```bash
onchainos leaderboard list --chain solana
```

**Output**: List of top trader addresses ranked by PnL or win rate with recent trade direction.

---

### 6. Run a full signal convergence scan

Core workflow: scan Polymarket for a trending topic, find the related Hyperliquid instrument, and check if signals align.

**When to use**: When the user says "scan for opportunities" or "what should I be looking at today".

**Steps**:

1. Fetch top Polymarket markets (Command 1)
2. Identify a market linked to a crypto asset such as BTC above 100k
3. Fetch Hyperliquid funding rate for that asset (Command 3)
4. Fetch OKX smart-money signals for the same asset (Command 5)
5. Compare signals:
   - Polymarket YES above 0.60 AND Hyperliquid funding negative means convergence bullish
   - Polymarket YES below 0.40 AND Hyperliquid funding positive means convergence bearish
   - Polymarket and Hyperliquid pointing opposite directions means divergence
6. Present a clear summary

**Output format**:
```
Signal Convergence Report
Market: Will BTC close above 100k this week?
Polymarket YES: 61% bullish
HL Funding rate: -0.012% per 8h shorts paying bullish
OKX Smart money: Net long bias on BTC
VERDICT: CONVERGENCE BULLISH
Confidence: High 3 of 3 sources aligned
```

---

### 7. Analyze a Polymarket wallet position history

Look up positions a wallet address holds on Polymarket.

**When to use**: When the user provides a Polygon wallet address and wants to know its open prediction market positions.

```bash
curl -s "https://data-api.polymarket.com/positions?user=<WALLET_ADDRESS>&sizeThreshold=.1&limit=20" \
  | python3 -c "
import json, sys
data = json.load(sys.stdin)
if not data:
    print('No positions found for this address.')
else:
    for p in data:
        print(f\"{p.get('market','?')} outcome: {p.get('outcome','?')} size: {p.get('size','?')}\")
"
```

**Output**: All open prediction market positions for the wallet including market question, outcome held, and size.

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| onchainos command not found | CLI not installed or not in PATH | Run npx skills add okx/onchainos-skills then export PATH |
| curl could not resolve host | No internet or DNS failure | Check network connectivity |
| JSON decode error | API returned empty response | Wait 10 seconds and retry |
| No positions found | Wallet has no open positions | Verify the address is a Polygon wallet for Polymarket |
| funding rate missing | Symbol not found on Hyperliquid | Use exact symbol name such as BTC ETH SOL |
| HTTP 403 from Hyperliquid | Rate limited | Wait 30 seconds and retry |

---

## Security Notices

- This skill is read-only. It does not place orders, execute transactions, or move any funds.
- No private keys or API keys are required for any command in this skill.
- Wallet addresses queried are public on-chain data only.
- Risk level: starter (read-only, no asset movement).

---

## Skill Routing

- To execute a trade on Polymarket use the Polymarket Basic Skill
- To execute a perpetual trade on Hyperliquid use the Hyperliquid Basic Skill
- To swap tokens use okx-dex-swap skill
- To check your own wallet balances use okx-wallet-portfolio skill
