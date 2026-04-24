---
name: kairos-floor-strategy
description: "Autonomous AI trading strategy for Hyperliquid perpetual futures with multi-timeframe confluence scoring, conviction-based sizing, and adaptive risk management"
version: "1.0.0"
author: "Kairos Lab"
tags:
  - trading
  - hyperliquid
  - perpetual-futures
  - ai-strategy
  - risk-management
  - autonomous
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. If an update is applied, re-read this SKILL.md before proceeding — the instructions may have changed.

```bash
# Check for skill updates (1-hour cache)
UPDATE_CACHE="$HOME/.plugin-store/update-cache/kairos-floor-strategy"
CACHE_MAX=3600
LOCAL_VER="1.0.0"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/kairos-floor-strategy/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: kairos-floor-strategy v$LOCAL_VER -> v$REMOTE_VER. Updating..."
  npx skills add okx/plugin-store --skill kairos-floor-strategy --yes --global 2>/dev/null || true
  echo "Updated kairos-floor-strategy to v$REMOTE_VER. Please re-read this SKILL.md."
fi
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/kairos-floor-strategy"
if [ ! -f "$REPORT_FLAG" ]; then
  mkdir -p "$HOME/.plugin-store/reported"
  # Device fingerprint → SHA256 → 32-char device ID
  DEV_RAW="$(hostname):$(uname -s):$(uname -m):$HOME"
  DEV_ID=$(echo -n "$DEV_RAW" | shasum -a 256 | head -c 32)
  # HMAC signature (obfuscated key, same as CLI binary)
  _K=$(echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | base64 -d 2>/dev/null || echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | openssl base64 -d)
  HMAC_SIG=$(echo -n "${_K}${DEV_ID}" | shasum -a 256 | head -c 8)
  DIV_ID="${DEV_ID}${HMAC_SIG}"
  unset _K
  # Report to Vercel stats
  curl -s -X POST "https://plugin-store-dun.vercel.app/install" \
    -H "Content-Type: application/json" \
    -d '{"name":"kairos-floor-strategy","version":"1.0.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"kairos-floor-strategy","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


# KAIROS FLOOR — Autonomous Trading Strategy

## Overview

KAIROS FLOOR is a multi-agent trading strategy for Hyperliquid perpetual futures.
It runs a 4-phase analysis pipeline before every trade: Market Regime → Signal Detection → Risk Validation → Execution.

The strategy uses 9-factor confluence scoring, conviction-based position sizing (5 tiers from SCOUT to APOCALYPSE),
and adaptive trailing stops with dollar-based step-lock protection.

**They sell tools. We run your fund.**

---

## IMPORTANT: Risk Disclaimer

⚠️ **This is an ADVANCED risk-level strategy that trades autonomously.**
- This strategy can and will lose money. Past performance does not guarantee future results.
- NEVER trade with funds you cannot afford to lose.
- The strategy includes a DRY-RUN mode (default) for paper trading.
- All positions have stop-loss protection. Maximum loss per trade is capped at 2% of capital.
- The author is not responsible for any financial losses incurred by using this strategy.
- This is NOT financial advice. This is experimental autonomous trading software.

---

## Pre-flight Checks

Before using this skill, the AI agent MUST verify:

1. **Hyperliquid Plugin is installed and accessible:**
   ```
   hyperliquid --version
   ```
   If not found, the user must install the Hyperliquid Plugin from the OKX Plugin Store.

2. **Check user balance on Hyperliquid:**
   ```
   hyperliquid balance
   ```

3. **Check current trading mode (dry-run by default):**
   ```
   node scripts/config.mjs status
   ```
   If the user has NOT explicitly said "go live" or "use real money", ALWAYS use dry-run mode.

4. **Verify market data access:**
   ```
   node scripts/market-data.mjs ping
   ```

---

## Strategy Architecture

The KAIROS engine runs 4 phases sequentially. ALL phases must pass for a trade to execute.

### Phase 1: SENTINEL — Market Regime (runs every 3 minutes)

Determines if the market is tradable and in which direction.

```
node scripts/kairos-engine.mjs sentinel
```

**Output**: JSON with regime (trending_up/trending_down/ranging/volatile), tradable (true/false), direction (LONG/SHORT/BOTH), strength (1-5).

**Gate**: If regime is "volatile" with ADX > 55 → skip trading cycle. Otherwise proceed.

### Phase 2: SCANNER — Setup Detection (runs every 2 minutes when tradable)

Analyzes symbols for entry opportunities using 9-factor confluence scoring.

```
node scripts/kairos-engine.mjs scan --symbols BTC,ETH,SOL
```

**9 Confluence Factors**:
1. EMA 9/21 alignment on 5m
2. ADX > 20 (trend exists)
3. RSI in sweet spot (25-75)
4. MACD histogram confirms direction
5. Volume above average
6. Multi-timeframe 15m EMA alignment
7. Multi-timeframe 1h EMA alignment
8. ATR breakout (last candle > 1.5× ATR)
9. Bollinger Band compression (width < 40th percentile)

**Minimum confluence**: 2/9 required for entry (challenge mode).
**Output**: Array of setups with symbol, direction, confidence (0-100), entry/SL/TP prices.

### Phase 3: GUARDIAN — Risk Validation (per setup)

Validates each setup against risk rules before execution.

```
node scripts/kairos-engine.mjs guardian --setup <SETUP_JSON>
```

**Rules checked**:
- Max 6 simultaneous positions
- No duplicate symbols
- Daily loss < 5% of capital
- Consecutive losses < 5
- Net direction lock (blocks opposing positions when 4+ open)
- Funding rate check (< 0.08% against position)

**Output**: GO (execute) or VETO (reject) with reasoning.

### Phase 4: EXECUTOR — Position Sizing & Order

Calculates conviction-based leverage and sizing, then executes via Hyperliquid Plugin.

```
node scripts/kairos-engine.mjs execute --setup <APPROVED_SETUP_JSON>
```

**Conviction Tiers** (based on confluence + confidence + regime):
| Tier | Score | Leverage | Risk/Trade |
|------|-------|----------|------------|
| SCOUT | 0-30 | 5x | 0.5% |
| STANDARD | 30-50 | 10x | 1.0% |
| CONFIDENT | 50-70 | 15x | 1.5% |
| CONVICTION | 70-85 | 20x | 2.0% |
| APOCALYPSE | 85+ | 25x | 2.0% |

**Execution** (uses Hyperliquid Plugin):
```
hyperliquid perp order BTC-PERP --side buy --size <SIZE> --leverage <LEV>
hyperliquid perp order BTC-PERP --side sell --size <SIZE> --trigger-price <SL_PRICE> --reduce-only --order-type stop
```

---

## Commands

### Full Auto Cycle (recommended)

Run one complete analysis + execution cycle (mode is read from persisted config — dry-run by default):

```
node scripts/kairos-engine.mjs cycle --symbols BTC,ETH,SOL
```

**When to use**: When the user says "analyze the market", "find trades", "run KAIROS", or "trade for me".
**Output**: Full pipeline results — regime, setups found, guardian decisions, orders placed (or simulated).

### Continuous Mode

Run the engine continuously, cycling every 2 minutes:

```
node scripts/kairos-engine.mjs auto --symbols BTC,ETH,SOL --duration 60
```

**When to use**: When the user says "trade autonomously", "run on autopilot", "keep trading for an hour".
**Duration**: In minutes. Default 60. Max 480 (8 hours).

### Check Positions

```
hyperliquid perp positions
```
Or view via the engine:
```
node scripts/kairos-engine.mjs stats
```

**When to use**: When the user asks "what positions do I have", "show my trades", "how am I doing".

### Manage Risk

```
node scripts/risk-manager.mjs check-positions
```

**When to use**: Every 30 seconds during active trading. Checks trailing stops, step-lock, time stops.
**Actions**: Tightens SL, triggers breakeven, exits on RSI extreme, step-locks dollar gains.

### Switch to Live Mode

```
node scripts/config.mjs set-mode live --confirm
```

**When to use**: ONLY when the user explicitly says "go live", "use real money", "trade for real".
**CRITICAL**: ALWAYS confirm with the user before running this command.
**ALWAYS ask**: "You're about to trade with real money on Hyperliquid. Are you absolutely sure? This strategy can lose money."

### Check Performance

```
node scripts/kairos-engine.mjs stats
```

**When to use**: When user asks "how's my PnL", "show results", "performance report".

### Check Config

```
node scripts/config.mjs status
```

### Market Data

```
node scripts/market-data.mjs market-overview
node scripts/market-data.mjs fear-greed
node scripts/market-data.mjs price BTC
```

---

## Risk Management

### Trailing Stop V3 (3-phase)

| Phase | Trigger | SL Position |
|-------|---------|-------------|
| INITIAL | Entry | Entry ± ATR × 2.0 |
| BREAKEVEN | Profit ≥ 0.7R | Entry + 0.15% buffer |
| TRAILING | Profit ≥ 1.2R | ATR-adaptive (current price ± ATR × 1.5) |

SL moves in the favorable direction only — never retraces.

### Step-Lock Protection

Every $2 of unrealized profit is locked. The SL can NEVER go below the last locked level.
Example: PnL reaches $6.40 → SL locks minimum at $6.00. If price drops, closed at ≥ $6.00.

Managed by:
```
node scripts/risk-manager.mjs check-positions
```

### RSI Exit Signal

When in profit > 0.3R:
- RSI > 69 (LONG) or RSI < 31 (SHORT) → tighten trailing aggressively
- RSI > 78 (LONG) or RSI < 22 (SHORT) → EXIT NOW (take profit before reversal)

### Circuit Breakers

| Protection | Threshold | Action |
|------------|-----------|--------|
| Max drawdown | 15% | Stop all trading |
| Daily loss | 5% | Block new entries for today |
| Consecutive losses | 5 | 30 min cooldown |
| Max positions | 6 | Block new entries |
| Time stop | 120 min | Close position automatically |

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Insufficient balance" | Not enough USDC on Hyperliquid | Ask user to deposit: `hyperliquid deposit USDC --amount <N>` |
| "Market data unavailable" | API rate limit or downtime | Wait 30 seconds and retry. Max 3 retries. |
| "Position limit reached" | 6 positions already open | Wait for a position to close or manually close one |
| "Leverage exceeds max" | Symbol max leverage < requested | Use the maximum allowed leverage for this symbol |
| "Order rejected" | Exchange error | Log the error, skip this cycle, retry next cycle |
| "Funding rate extreme" | Paying > 0.08% funding | Skip this symbol, try another |
| "hyperliquid CLI not found" | Plugin not installed | Install Hyperliquid Plugin via OKX Plugin Store |

---

## Security Notices

- This plugin operates at the **ADVANCED** risk level
- **DRY-RUN mode is the default** — no real trades until explicitly enabled with `config.mjs set-mode live --confirm`
- All positions have stop-loss protection
- Maximum loss per trade is capped at 2% of capital
- The strategy never handles private keys directly — all signing is done via Hyperliquid Plugin
- External API calls:
  - `api.hyperliquid.xyz` (market data + order execution via CLI)
  - `api.coingecko.com` (market dominance data)
  - `api.alternative.me` (Fear & Greed Index)

---

## Skill Routing

- For token swaps on other chains → use `okx-dex-swap` skill
- For portfolio overview across chains → use `okx-wallet-portfolio` skill
- For token security checks → use `okx-security` skill
- For Hyperliquid spot trading → use the Hyperliquid Plugin directly
- For prediction markets → use `polymarket-plugin` skill

---

## Quick Start Examples

```
User: "Analyze the market and find me some trades"
Agent: node scripts/kairos-engine.mjs cycle --symbols BTC,ETH,SOL

User: "What's the market regime right now?"
Agent: node scripts/kairos-engine.mjs sentinel

User: "Run KAIROS for the next 2 hours"
Agent: node scripts/kairos-engine.mjs auto --duration 120

User: "Show me my open positions"
Agent: hyperliquid perp positions
Then: node scripts/kairos-engine.mjs stats

User: "I want to trade with real money"
Agent: CONFIRM with user first, then:
       node scripts/config.mjs set-mode live --confirm

User: "How's the market feeling?"
Agent: node scripts/market-data.mjs fear-greed
       node scripts/market-data.mjs market-overview
```
