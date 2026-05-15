---
name: xlayer-bridge-yield
description: "Bridge assets to X Layer and auto-optimize stablecoin yield farming with risk management"
version: "1.0.0"
author: "oliva9595"
tags:
  - xlayer
  - bridge
  - yield
  - stablecoin
  - defi
---

# X Layer Bridge + Yield Optimizer

## Overview

This skill enables the AI agent to bridge stablecoins (USDC, USDT, DAI) from any supported chain to X Layer (OKX's L2, chain ID `196`) and automatically find, enter, and monitor the best yield farming positions. It combines cross-chain bridge route optimization with on-chain yield scanning to provide a seamless "bridge and farm" experience.

**Key capabilities:**
- Compare bridge routes across multiple paths for cost and speed
- Scan and rank X Layer DeFi yield pools by APY, TVL, and risk
- Execute the full bridge-to-farm pipeline with safety checks
- Monitor active positions with depeg alerts and rebalance suggestions

## Pre-flight Checks

Before using this skill, ensure:

1. The `onchainos` CLI is installed and configured:
   ```bash
   npx skills add okx/onchainos-skills
   ```
2. Python 3.8+ is available (for helper scripts):
   ```bash
   python3 --version
   ```
3. The user has a funded wallet on the source chain (Ethereum, BSC, Polygon, Arbitrum, or Solana)
4. The `onchainos wallet` is logged in and authenticated

## Safety Configuration

This plugin operates in **dry-run mode by default**. All commands simulate transactions without executing unless the user explicitly confirms.

### Default Safety Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `mode` | `dry-run` | No real transactions unless user says "execute" or "confirm" |
| `max_amount` | `1000` | Maximum USD value per single transaction |
| `stop_loss` | `-5%` | Alert + suggest exit if position drops below this |
| `min_pool_tvl` | `50000` | Skip pools with TVL below $50,000 |
| `min_pool_age_days` | `30` | Skip pools younger than 30 days |
| `depeg_alert_threshold` | `0.005` | Alert if stablecoin deviates > 0.5% from peg |

The user can override any parameter by specifying it in their request (e.g., "set max amount to 5000").

---

## Commands

### 1. Bridge Route Scan

Compares cross-chain bridge routes to find the cheapest and fastest path to X Layer.

```bash
onchainos swap quote --from <TOKEN> --to <TOKEN> --amount <AMOUNT> --chain <SOURCE_CHAIN> --dest-chain 196 --format json
```

**When to use**: When the user asks "find cheapest bridge to X Layer", "compare bridge routes", or "how much does it cost to bridge USDC to X Layer".

**Workflow**:
1. Identify the source chain and token from user's request
2. Check user's balance on source chain:
   ```bash
   onchainos portfolio all-balances --address <WALLET> --chain <SOURCE_CHAIN> --format json
   ```
3. Get cross-chain quotes for the amount:
   ```bash
   onchainos swap quote --from <TOKEN_ADDRESS> --to <XLAYER_TOKEN_ADDRESS> --amount <AMOUNT> --chain <SOURCE_CHAIN> --dest-chain 196 --format json
   ```
4. Run the bridge optimizer script to rank routes:
   ```bash
   python3 scripts/bridge_optimizer.py --quotes '<QUOTE_JSON>'
   ```
5. Present results as a table:

| Route | Fee | Est. Time | Slippage | Score |
|-------|-----|-----------|----------|-------|
| Route A | $0.50 | ~2 min | 0.01% | 9.5 |
| Route B | $1.20 | ~1 min | 0.02% | 8.8 |

**Output**: Ranked table of bridge routes with fees, estimated time, slippage, and composite score.

**If no routes found**: Tell the user that the token/amount may not be supported for cross-chain to X Layer. Suggest trying a different amount or token.

---

### 2. Yield Pool Scan

Scans DeFi protocols on X Layer to find the best yield farming opportunities for stablecoins.

**When to use**: When the user asks "show yield pools on X Layer", "best APY on X Layer", or "where to farm USDC on X Layer".

**Workflow**:
1. Search for stablecoin-related tokens and pools on X Layer:
   ```bash
   onchainos token search --query USDC --chain 196 --format json
   onchainos token search --query USDT --chain 196 --format json
   ```
2. Get market data for discovered pools:
   ```bash
   onchainos market price --address <POOL_TOKEN> --chain 196 --format json
   ```
3. Run security scan on each pool token:
   ```bash
   onchainos security token-scan --address <POOL_TOKEN> --chain 196 --format json
   ```
4. Run the yield scanner script to rank pools:
   ```bash
   python3 scripts/yield_scanner.py --tokens '<TOKEN_DATA_JSON>' --market '<MARKET_DATA_JSON>' --security '<SECURITY_JSON>'
   ```
5. Present results:

| Pool | Protocol | APY | TVL | Risk | Score |
|------|----------|-----|-----|------|-------|
| USDC/USDT | DEX-A | 8.5% | $2.1M | LOW | 9.2 |
| USDC/OKB | DEX-B | 12.3% | $850K | MEDIUM | 7.8 |

**Output**: Ranked table of yield pools with APY, TVL, risk level, and composite score.

**Filtering rules**:
- Skip pools with TVL < `min_pool_tvl` (default $50,000)
- Skip pools where security scan returns CRITICAL risk
- Flag pools younger than `min_pool_age_days` as HIGH risk

---

### 3. Bridge and Farm (Full Pipeline)

Executes the complete workflow: bridge assets from source chain to X Layer, then deposit into the best yield pool.

**When to use**: When the user asks "bridge and farm", "bridge 100 USDC to X Layer and find yield", or "move my USDT to X Layer for farming".

**Workflow**:
1. **Validate amount**: Ensure amount <= `max_amount`. If exceeded, warn user and ask for confirmation.
2. **Run Bridge Route Scan** (Command 1) to find the best route.
3. **Run Yield Pool Scan** (Command 2) to find the best pool.
4. **Security pre-check**:
   ```bash
   onchainos security token-scan --address <TARGET_POOL_TOKEN> --chain 196 --format json
   ```
   If risk level is CRITICAL → **STOP and warn user. Do not proceed.**
5. **Present plan to user** — show:
   - Bridge route (fee, time, slippage)
   - Target yield pool (APY, TVL, risk)
   - Total estimated cost
   - Expected daily/monthly yield
6. **WAIT FOR USER CONFIRMATION** — Do not proceed without explicit "confirm", "execute", or "yes".
7. **Execute bridge**:
   ```bash
   onchainos swap swap --from <TOKEN> --to <XLAYER_TOKEN> --amount <AMOUNT> --chain <SOURCE_CHAIN> --dest-chain 196 --format json
   ```
8. **Wait for bridge completion** — poll status every 30 seconds:
   ```bash
   onchainos portfolio all-balances --chain 196 --format json
   ```
9. **Execute yield deposit** (if the pool requires a swap first):
   ```bash
   onchainos swap swap --from <BRIDGED_TOKEN> --to <POOL_TOKEN> --amount <RECEIVED_AMOUNT> --chain 196 --format json
   ```
10. **Report result**: Transaction hash, amount deposited, expected APY, position ID.

**If bridge fails**: Show error, suggest retry. Do not auto-retry.
**If in dry-run mode**: Show the full plan with simulated values but do NOT execute steps 7-10. Print `[DRY-RUN] No transactions were executed.`

---

### 4. Position Monitor

Checks the current status of yield farming positions on X Layer.

**When to use**: When the user asks "check my positions", "how is my X Layer farming doing", or "show my yield status".

**Workflow**:
1. Get current balances on X Layer:
   ```bash
   onchainos portfolio all-balances --chain 196 --format json
   ```
2. Get current market prices for held tokens:
   ```bash
   onchainos market price --address <TOKEN> --chain 196 --format json
   ```
3. Run position monitor script:
   ```bash
   python3 scripts/position_monitor.py --balances '<BALANCES_JSON>' --prices '<PRICES_JSON>'
   ```
4. Present position summary:

| Token | Amount | Value | Change | Status |
|-------|--------|-------|--------|--------|
| USDC | 100.5 | $100.50 | +$0.50 | HEALTHY |
| LP-USDC-USDT | 95.2 | $95.18 | -$4.82 | ⚠️ CHECK |

5. **Alert conditions** (auto-triggered):
   - Depeg alert: stablecoin price deviates > 0.5% from $1.00
   - Loss alert: position value dropped > `stop_loss` threshold
   - APY change: significant APY decrease detected

**Output**: Position table with alerts and actionable suggestions.

---

## Examples

### Example 1: Quick Bridge Scan

**User**: "Find the cheapest way to bridge 500 USDC from Ethereum to X Layer"

1. Check balance: `onchainos portfolio all-balances --chain 1 --format json`
2. Get quotes: `onchainos swap quote --from 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 --to 0x... --amount 500000000 --chain 1 --dest-chain 196 --format json`
3. Run optimizer: `python3 scripts/bridge_optimizer.py --quotes '<json>'`
4. Present: "The cheapest route costs $0.45 via [Bridge], arriving in ~3 minutes."

### Example 2: Full Bridge and Farm

**User**: "Bridge 200 USDT to X Layer and farm the best yield"

1. Scan bridge routes → best route costs $0.30
2. Scan yield pools → best pool is USDT/USDC at 7.2% APY
3. Present plan: "Bridge 200 USDT ($0.30 fee) → deposit in USDT/USDC pool (7.2% APY, ~$0.04/day yield). Confirm?"
4. User says "yes" → execute bridge → wait → deposit
5. Report: "Done! 199.70 USDT deposited. Expected yield: $1.18/month."

### Example 3: Position Check with Alert

**User**: "Check my X Layer farming positions"

1. Get balances on chain 196
2. Calculate P/L
3. "Your USDC/USDT position: $198.50 (entry $200.00, -0.75%). APY currently 6.8%. Status: HEALTHY. No alerts."

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Insufficient balance on source chain" | Not enough tokens to bridge | Ask user to check their balance or reduce the amount |
| "No bridge routes found" | Token/amount not supported for cross-chain | Suggest trying USDC or USDT, or a different source chain |
| "Bridge transaction failed" | Network congestion or insufficient gas | Ask user to retry. Do not auto-retry |
| "Pool token security: CRITICAL" | Target pool has critical security issues | STOP immediately. Warn user. Suggest alternative pools |
| "Amount exceeds max_amount" | User requested more than safety limit | Show current limit, ask user to confirm override or reduce amount |
| "onchainos not found" | CLI not installed | Run `npx skills add okx/onchainos-skills` |
| "Wallet not authenticated" | Not logged in | Run `onchainos wallet login` |
| "Python not found" | Python 3 not installed | Ask user to install Python 3.8+ |

---

## Security Notices

> **RISK DISCLAIMER**: This plugin interacts with DeFi protocols and performs on-chain transactions. DeFi carries inherent risks including but not limited to: smart contract vulnerabilities, impermanent loss, stablecoin depeg events, bridge exploits, and loss of funds. Past APY does not guarantee future returns. Always review transactions before confirming. This plugin is provided as-is with no warranty. Use at your own risk.

- **Risk Level**: Advanced — this plugin can execute transactions autonomously when not in dry-run mode
- **Default Mode**: Dry-run (no real transactions unless user explicitly confirms)
- **Private Keys**: This plugin NEVER handles, stores, or requests private keys. All signing is done through onchainos secure wallet (TEE)
- **Safety Checks**: Pre-execution security scans are mandatory and cannot be bypassed
- **Maximum Limits**: Configurable per-transaction caps prevent accidental large trades
- **Stop-Loss**: Automated alerts when positions exceed loss thresholds

## Skill Routing

- For token swaps only → use `okx-dex-swap` skill
- For wallet balances → use `okx-wallet-portfolio` skill
- For security scanning → use `okx-security` skill
- For pre-trade risk assessment → use `agent-risk-firewall` skill
- For smart money signals on X Layer → use `xlayer-alpha-hunter` skill (if installed)
