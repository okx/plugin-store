---
name: pm-perp-momentum
description: "Polymarket-to-Hyperliquid momentum strategy for Onchain OS."
version: "0.1.0"
author: "Taylan Bal"
tags:
  - polymarket
  - hyperliquid
  - strategy
  - trading
---

# pm-perp-momentum

## Overview

`pm-perp-momentum` is a strategy plugin that converts Polymarket event probability momentum into Hyperliquid perpetual futures execution. It uses dependent plugins for protocol integration, enforces strategy attribution on trading operations, and produces deterministic proof artifacts for auditability.

The primary execution commands are `dry-run` and `live`. The utility commands are `resolve-market`, `suggest-markets`, `export-proof`, and `replay`.
The integrity command is `verify-proof`.
Maintainer GitHub handle: `midasbal`.

Architecture note: utility commands (`resolve-market`, `suggest-markets`) use direct API access for discovery speed and operator UX, while all strategy execution and trading paths (`dry-run`, `live`) are strictly mediated through the official Basic Skill plugins (`polymarket-plugin`, `hyperliquid-plugin`) for challenge compliance and attribution.

## Pre-flight Checks

Before running this skill, verify the following:

1. `polymarket-plugin` and `hyperliquid-plugin` are installed and available on PATH.
2. The runtime has Node.js `>= 20.10.0` and project dependencies installed.
3. The user has selected a valid Polymarket token ID and Hyperliquid perp market.
4. The user understands this is an advanced trading strategy and has reviewed the risk disclaimer.
5. **Environment Warning**: **CRITICAL: KPI attribution and leaderboard points are ONLY earned when this Skill is executed within the official Onchain OS / Agentic Wallet environment.**
6. For live mode, the user explicitly accepts the live safety gate:
   - `--confirm-live`
   - `--risk-ack "I acknowledge leveraged trading risk and accept full responsibility."`

## Commands

### Run Dry Simulation

```bash
node dist/src/index.js dry-run \
  --pm-market "<TOKEN_ID>" \
  --signal-side YES \
  --entry-threshold 70 \
  --exit-threshold 60 \
  --dwell-seconds 20 \
  --perp ETH \
  --side LONG \
  --notional-usd 2000 \
  --leverage 5 \
  --stop-loss-pct 8 \
  --leaderboard-mode volume-max \
  --state-db ./fixtures/pm-dryrun.sqlite
```

**When to use**: Validate thresholds, slicing, and exits without placing live orders.  
**Output**: Signal logs, simulated order actions, and replay/proof data in SQLite.

### Run Guarded Live Strategy

```bash
node dist/src/index.js live \
  --pm-market "<TOKEN_ID>" \
  --signal-side YES \
  --entry-threshold 70 \
  --exit-threshold 60 \
  --dwell-seconds 20 \
  --perp ETH \
  --side LONG \
  --notional-usd 2000 \
  --leverage 5 \
  --stop-loss-pct 8 \
  --leaderboard-mode volume-max \
  --daily-loss-limit-usd 1500 \
  --consecutive-loss-limit 5 \
  --state-db ./fixtures/pm-live.sqlite \
  --resume-open-position \
  --confirm-live \
  --risk-ack "I acknowledge leveraged trading risk and accept full responsibility."
```

**When to use**: Execute the strategy against live plugin-integrated market data and order flow.  
**Output**: Live trade decisions, mark-risk checks, and realized risk-state updates.

### Resolve Market Token IDs

```bash
node dist/src/index.js resolve-market --input "<slug-or-url>" --json
```

**When to use**: Convert a Polymarket URL or slug into token IDs for strategy execution.  
**Output**: Market metadata and token ID list.

### Suggest Active Markets

```bash
node dist/src/index.js suggest-markets --limit 10 --min-liquidity 5000 --json
```

**When to use**: Discover liquid candidate markets for strategy deployment.  
**Output**: Ranked active markets with liquidity, volume, and token IDs.

### Export Proof Artifacts

```bash
node dist/src/index.js export-proof \
  --state-db ./fixtures/pm-live.sqlite \
  --format json \
  --output ./fixtures/proof-live.json
```

**When to use**: Generate submission-ready evidence from a completed run.  
**Output**: Run metadata, positions, events, and replay ticks in JSON or CSV format.

### Replay Stored Run Deterministically

```bash
node dist/src/index.js replay --state-db ./fixtures/pm-live.sqlite --json
```

**When to use**: Reconstruct decision flow from recorded ticks for audit and verification.  
**Output**: Replay summary and deterministic action timeline.

### Verify Proof Integrity

```bash
node dist/src/index.js verify-proof --state-db ./fixtures/pm-live.sqlite --json
```

**When to use**: Verify run fingerprint and hash-chain integrity before submission.  
**Output**: Pass/fail report with any detected integrity issues.

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| `Live mode requires --confirm-live` | Live command missing mandatory safety gate | Add `--confirm-live` and rerun |
| `Live mode requires --risk-ack` | Missing or incorrect risk acknowledgment string | Provide the exact required acknowledgment string |
| `Failed to fetch Polymarket data through polymarket-plugin` | Plugin command mismatch, plugin unavailable, or transport failure | Verify plugin installation and supported read commands |
| `Failed to fetch Hyperliquid mark price` | Mark price query command not supported or temporary plugin/provider failure | Verify `hyperliquid-plugin` commands and retry |
| `Entry was blocked due to risk guardrails` | Daily loss limit or consecutive-loss breaker threshold exceeded | Reduce risk, wait for reset, or start a new controlled session |
| `No replay ticks were stored for this run` | Tick recording disabled or empty session | Run dry/live without `--no-record-ticks` and retry replay |
| `Unsupported argument: --...` | Typo or unsupported CLI flag | Use `--help` and rerun with supported arguments only |
| `Proof verification failed` | Event chain mismatch or fingerprint mismatch | Re-export from untampered DB and investigate local modifications |

## Security Notices

- This is an **advanced** autonomous trading strategy.
- Dry-run mode should be executed before any live deployment.
- Stop-loss is driven by Hyperliquid mark-price risk checks, independent of Polymarket signal availability.
- Daily loss and consecutive-loss protections are persisted and enforced across restarts.
- Open-position recovery is opt-in through `--resume-open-position` to avoid cross-session state surprises.
- All trading subprocess calls to dependent plugins include `--strategy-id pm-perp-momentum` via centralized command wrapping.

**Risk Disclaimer**: Leveraged perpetual trading can cause rapid and substantial losses. Market volatility, spread expansion, slippage, and execution latency can degrade performance. This plugin is provided as-is with no guarantee of profitability. Users are solely responsible for all live trading decisions and capital risk.
