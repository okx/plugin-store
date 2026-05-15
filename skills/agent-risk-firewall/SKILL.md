---
name: agent-risk-firewall
description: "Pre-trade risk firewall for Agentic Wallet swaps on X Layer and Solana"
version: "1.2.0"
author: "Agent Risk Firewall Contributors"
tags:
  - security
  - risk
  - agentic-wallet
  - xlayer
  - solana
  - trading
---

# Agent Risk Firewall

## Overview

Agent Risk Firewall is a pre-trade guardian for Agentic Wallet workflows. Use it before any X Layer or Solana swap, token buy, token sell, or approval when an agent has quote or transaction context and is about to ask the user to sign.

The firewall does not sign transactions, broadcast transactions, revoke approvals, or execute swaps. It normalizes the proposed operation, calls OKX OnchainOS security and simulation commands when available, accepts optional external evidence from other plugins, applies a deterministic policy profile, and returns `allow`, `warn`, or `block`.

## Pre-flight Checks

Before using this skill, ensure:

1. Python 3.8+ is available.
2. The `agent-risk-firewall` command is installed from this plugin.
3. For live OKX checks, install OnchainOS skills with `npx skills add okx/onchainos-skills`.
4. For production usage, configure personal OKX credentials through the OnchainOS environment variables. Never commit `.env` files or API keys.

If `onchainos` is not installed or a scan fails, treat that as incomplete verification. In `balanced`, `competition`, and `degen-small-size`, unavailable scan data returns at least `warn`. In `strict`, unavailable scan data returns `block`.

## Commands

### Check a proposed trade or approval

```bash
agent-risk-firewall check --input request.json --format json
```

**When to use**: Run this before any X Layer or Solana swap, token buy, token sell, or approval when the agent has quote or transaction context and is about to request a signature.

**Output**: JSON containing `verdict`, `riskScore`, `requiresUserConfirmation`, `reasons`, normalized `evidence`, `audit`, and `safeNextStep`.

**Example**:

```bash
agent-risk-firewall check --input request.json --format json
```

Use `--input -` to read JSON from stdin.

Required input shape:

```json
{
  "chain": "xlayer",
  "operation": "swap",
  "walletAddress": "0x0000000000000000000000000000000000000000",
  "tokenIn": {
    "address": "0x0000000000000000000000000000000000000001",
    "symbol": "USDC",
    "decimals": 6
  },
  "tokenOut": {
    "address": "0x0000000000000000000000000000000000000002",
    "symbol": "TOKEN",
    "decimals": 18
  },
  "amountIn": "100",
  "amountInUsd": 100,
  "walletValueUsd": 1000,
  "quote": {
    "expectedOut": "12345",
    "slippagePct": 1,
    "priceImpactPct": 0.8,
    "route": ["OKX DEX"],
    "venue": "okx-dex"
  },
  "tx": {
    "from": "0x0000000000000000000000000000000000000000",
    "to": "0x0000000000000000000000000000000000000003",
    "data": "0x",
    "value": "0"
  },
  "approval": {
    "spender": "0x0000000000000000000000000000000000000004",
    "spenderType": "contract",
    "isUnlimited": false
  },
  "externalEvidence": {
    "goplus": {},
    "birdeye": {},
    "rootdata": {}
  },
  "competition": {
    "activityName": "Selected Agentic Trading competition",
    "active": true,
    "joined": true,
    "supportedChains": ["xlayer", "solana"],
    "primaryChain": "xlayer",
    "minParticipationUsd": 25,
    "minLeaderboardUsd": 100,
    "minWalletBalanceUsd": 100,
    "eligibleTokenTradeRequired": true,
    "disallowedPairClasses": ["stable-stable", "stable-native", "native-native", "native-wrapped-native"]
  },
  "policyProfile": "competition"
}
```

Output:

```json
{
  "verdict": "warn",
  "riskScore": 60,
  "requiresUserConfirmation": true,
  "reasons": [
    {
      "code": "TOKEN_HIGH",
      "severity": "warn",
      "message": "Target token has HIGH risk."
    }
  ],
  "evidence": {},
  "audit": {
    "decisionId": "arf_0123456789abcdef",
    "policyProfile": "balanced",
    "policyVersion": "1.2.0",
    "evidenceHash": "64-character-sha256"
  },
  "safeNextStep": "Show the warning reasons and ask the user for explicit confirmation before signing."
}
```

Agent behavior contract:

- `allow`: the agent may continue the normal signing or broadcast flow only if the user already requested execution.
- `warn`: the agent must show the warning reasons, quote details, and `audit.decisionId`, then require explicit user confirmation before continuing.
- `block`: the agent must stop. It must not ask the user to sign, must not broadcast, and must recommend canceling or revising the operation.

## Policy Profiles

Use `policyProfile` to tune the firewall for the agent workflow:

| Profile | Use case | Key behavior |
|---|---|---|
| `balanced` | Default retail agentic trading guardrail | Blocks critical signals, warns on elevated slippage/price impact, allows normal safe trades. |
| `strict` | High-safety mode for larger or less trusted workflows | Blocks unavailable scans, token `HIGH`, EOA spenders, and tighter slippage/price impact thresholds. |
| `competition` | OKX Agentic Trading style workflows | Requires competition preflight context, warns when registration or thresholds are incomplete, blocks inactive or unsupported-chain trades, and blocks stablecoin/native-only pairs. |
| `degen-small-size` | Small-size meme/token exploration | Allows higher slippage/price impact but caps trade size at 25 USD and 3% wallet exposure. |

## Competition Mode Enhancer

For OKX Agentic Trading competition workflows, run the firewall with `policyProfile: "competition"` after the agent has fetched competition detail and user-status.

The agent should pass a `competition` object with:

- `active`: whether the competition is active.
- `joined`: whether the wallet has registered.
- `supportedChains`: normalized chains that count for the competition. Until OKX exposes a backend multi-chain field, agents should treat Solana plus the competition primary chain as supported.
- `minParticipationUsd`, `minLeaderboardUsd`, `minWalletBalanceUsd`: thresholds parsed from competition rules when available.
- `eligibleTokenTradeRequired`: `true` when stablecoin/native-only trades should not count as eligible competition trades.
- `disallowedPairClasses`: pair classes such as `stable-stable`, `stable-native`, `native-native`, and `native-wrapped-native`.

Competition verdict behavior:

| Signal | Result |
|---|---|
| Missing competition context | `warn` |
| Competition inactive or ended | `block` |
| Requested chain not in `supportedChains` | `block` |
| Wallet not joined, or join status missing | `warn` |
| Trade amount or wallet value below competition thresholds | `warn` |
| Stablecoin/native-only pair | `block` |

Internal competition IDs can exist in tool context, but do not show them in user-facing messages.

## External Evidence

Other plugins can pass optional evidence into `externalEvidence` without this firewall calling third-party APIs directly:

- `goplus`: token/address security fields such as `riskLevel`, `is_honeypot`, `is_blacklisted`, `buy_tax`, `sell_tax`.
- `birdeye`: liquidity and holder distribution fields such as `liquidityUsd`, `top10HolderPercent`.
- `rootdata`: project intelligence fields such as `riskLevel`, `tags`, `labels`, or `riskTags`.

Critical external evidence can upgrade a result to `block`; high or incomplete evidence usually upgrades to `warn`.

## Approval-Specific Checks

For `operation: "approval"`, include an `approval` object when available:

```json
{
  "spender": "0x0000000000000000000000000000000000000004",
  "spenderType": "eoa",
  "isUnlimited": true,
  "allowedSpenders": [],
  "blockedSpenders": []
}
```

The firewall checks for missing spender, spender address mismatch, explicitly blocked spender, spender not in a provided allowlist, EOA spender, and unlimited allowance.

## Compatibility Examples

Trading strategy plugins can use this firewall as a pre-sign middleware:

```text
xlayer-alpha-hunter -> onchainos swap swap -> agent-risk-firewall check -> user confirmation or cancel -> onchainos swap execute
smart-tradex -> quote/unsigned tx -> agent-risk-firewall check -> allow/warn/block gate
otto-alpha-sniper -> externalEvidence + tx context -> agent-risk-firewall check -> final confirmation
```

The strategy plugin keeps its alpha logic. Agent Risk Firewall owns the pre-sign risk gate.

### Show the active policy

```bash
agent-risk-firewall policy --profile balanced
```

**When to use**: Run this when the user asks what thresholds the firewall applies, or before integrating the firewall into another trading skill.

**Output**: JSON describing supported chains, max trade size, wallet exposure cap, slippage thresholds, price impact thresholds, and low-liquidity threshold.

**Example**:

```bash
agent-risk-firewall policy --profile balanced
agent-risk-firewall policy --profile strict
agent-risk-firewall policy --profile competition
agent-risk-firewall policy --profile degen-small-size
```

### Run a local self-test

```bash
agent-risk-firewall self-test
```

**When to use**: Run this after installation or before submitting a PR to verify that the CLI, policy engine, and JSON renderer work without live assets or external scans.

**Output**: JSON with `status: pass` or `status: fail` plus three fixture verdicts for allow, warn, and block cases.

**Example**:

```bash
agent-risk-firewall self-test
```

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `INVALID_JSON` | Input is not valid JSON | Fix the input file or stdin payload. |
| `INVALID_INPUT` | Required fields are missing or malformed | Add `chain`, `operation`, `walletAddress`, token context, and quote/tx details. |
| `ONCHAINOS_UNAVAILABLE` | The `onchainos` CLI is not installed | Install `okx/onchainos-skills`, then retry. |
| `SCAN_TIMEOUT` | A live OKX scan timed out | Retry once; if still unavailable, treat the result as incomplete verification. |
| `UNSUPPORTED_CHAIN` | Chain is outside MVP scope | Use `xlayer` or `solana`. |

## Security Notices

- This plugin never asks for, stores, or handles private keys or seed phrases.
- This plugin never signs or broadcasts transactions.
- This plugin is a guardrail, not a guarantee of safety. Onchain data, external APIs, and simulations can be incomplete or stale.
- Do not override a `block` verdict. A `block` means the agent must cancel the proposed operation.
- `warn` requires explicit user confirmation before signing.
- Trading and DeFi activity can cause loss of funds. Use dry-run and small amounts first.
