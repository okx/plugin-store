---
name: sorin-skill
description: Use when the user asks crypto-related questions about a token, pool, chain, protocol, or project and the agent should answer with Sorin's DeFi gateway using clear, data-backed analysis.
version: "1.0.1"
author: Sahara AI
tags:
  - defi
  - crypto
  - analytics
  - tokens
  - yield
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Version check (auto-injected)

> **AI Agent Instruction**: Run this version check before executing any commands from this skill. The check is a read-only fetch from `raw.githubusercontent.com` — it does **not** install anything. If it reports `Update available`, you MUST stop, surface the message to the user verbatim, and obtain explicit confirmation before running the suggested `npx skills add …` command. Do not run the update silently. After a confirmed update completes, re-read this SKILL.md before proceeding.

```bash
# Check for skill updates (1-hour cache).
# Network scope: this block calls raw.githubusercontent.com for the version probe only.
# It does NOT install anything; install requires user-confirmed `npx skills add` below.
UPDATE_CACHE="$HOME/.plugin-store/update-cache/sorin-skill"
CACHE_MAX=3600
LOCAL_VER="1.0.1"
DO_CHECK=true

if [ -f "$UPDATE_CACHE" ]; then
  CACHE_MOD=$(stat -f %m "$UPDATE_CACHE" 2>/dev/null || stat -c %Y "$UPDATE_CACHE" 2>/dev/null || echo 0)
  NOW=$(date +%s)
  AGE=$(( NOW - CACHE_MOD ))
  [ "$AGE" -lt "$CACHE_MAX" ] && DO_CHECK=false
fi

if [ "$DO_CHECK" = true ]; then
  REMOTE_VER=$(curl -sf --max-time 3 "https://raw.githubusercontent.com/okx/plugin-store/main/skills/sorin-skill/plugin.yaml" | grep '^version' | head -1 | tr -d '"' | awk '{print $2}')
  if [ -n "$REMOTE_VER" ]; then
    mkdir -p "$HOME/.plugin-store/update-cache"
    echo "$REMOTE_VER" > "$UPDATE_CACHE"
  fi
fi

REMOTE_VER=$(cat "$UPDATE_CACHE" 2>/dev/null || echo "$LOCAL_VER")
if [ "$REMOTE_VER" != "$LOCAL_VER" ]; then
  echo "Update available: sorin-skill v$LOCAL_VER -> v$REMOTE_VER."
  echo "ACTION REQUIRED: ask the user to confirm before running:"
  echo "  npx skills add okx/plugin-store --skill sorin-skill --global"
  echo "(This contacts the npm registry and github.com/okx/plugin-store and overwrites this skill. Do NOT auto-run.)"
fi
```

---


# Sorin Skill

## Overview

Sorin Skill helps answer crypto-related questions about tokens, pools, chains, protocols, and projects through Sahara's Sorin DeFi AI Services Gateway.
It identifies the user's intent, selects the most relevant gateway endpoint, and returns concise, data-backed analysis with assumptions and risks.

## Gateway

- Base URL: `https://defi-tools-proxy.saharaa.info`
- API key env var: `DEFI_TOOLS_API_KEY`
- Auth header: `Authorization: Bearer ${DEFI_TOOLS_API_KEY}`
- Accept header: `accept: text/plain`

Default request template:

```bash
GET https://defi-tools-proxy.saharaa.info/<path>?<query>
Headers:
accept: text/plain
Authorization: Bearer ${DEFI_TOOLS_API_KEY}
```

Pool analysis example:

```bash
GET https://defi-tools-proxy.saharaa.info/pool/analysis?chain=Ethereum
Headers:
accept: text/plain
Authorization: Bearer ${DEFI_TOOLS_API_KEY}
```

## Quick Start

1. Run `sorin-skill quickstart`.
2. Verify `DEFI_TOOLS_API_KEY` is set in the agent runtime.
3. Check gateway connectivity using the pool analysis example.
4. If successful, confirm Sorin Skill is ready and suggest a token, pool, chain, protocol, or project analysis prompt.
5. If failed, return the gateway status and exact error message.

## Intention Routing

1. Detect user intention from natural language.
2. Select exactly one primary API first (add secondary APIs only when needed).
3. Validate required inputs. If missing, ask only the minimum follow-up question.
4. Call API and parse `success`, `data`, `error`.

## Intention -> API Mapping

### 1) Token fundamentals / price / holders / technicals

- API: `tokenTool`
- Use when: user asks token market analysis (macro liquidity + micro market structure).
- Input:
  - `token_symbol` (required): token symbol, e.g. `BTC`, `ETH`
  - `quote_currency` (optional, default `USDT`): quote currency for the pair
- Request:

```bash
GET https://defi-tools-proxy.saharaa.info/token/analysis?token_symbol=ETH&quote_currency=USDT
Headers:
accept: text/plain
Authorization: Bearer ${DEFI_TOOLS_API_KEY}
```

### 2) Yield/staking pool discovery and scoring

- API: `poolTool`
- Use when: user asks where to stake, best APY pools, pool comparison, TVL/APY trends.
- Input (all optional filters):
  - `chain` (optional): chain id string, e.g. `1`, `56`
  - `project` (optional): project slug, e.g. `lido`, `aave-v3`
  - `protocol` (e.g. `lido`, `aave-v3`)
  - `token_symbol` (e.g. `ETH`, `USDC`)
  - `pool_id` (optional): unique pool identifier
  - `pool_category` (optional): pool category filter
- Request:

```bash
GET https://defi-tools-proxy.saharaa.info/pool/analysis?chain=Ethereum&protocol=lido&token_symbol=ETH
Headers:
accept: text/plain
Authorization: Bearer ${DEFI_TOOLS_API_KEY}
```

### 3) Chain-level DEX and TVL landscape

- API: `chainTool`
- Use when: user asks chain DEX volume trends, TVL changes, protocol dominance on a chain.
- Input (optional, provide one when possible):
  - `chainId` (integer): chain id, e.g. `1`
  - `chainName` (string): chain name, e.g. `Ethereum`
- Request:

```bash
GET https://defi-tools-proxy.saharaa.info/chain/analysis?chainName=Ethereum
Headers:
accept: text/plain
Authorization: Bearer ${DEFI_TOOLS_API_KEY}
```

### 4) Protocol-level financial analysis

- API: `protocolTool`
- Use when: user asks protocol TVL/fees/revenue/capital flow and comprehensive metrics.
- Request:

```bash
GET https://defi-tools-proxy.saharaa.info/protocol/analysis?protocol=aave
Headers:
accept: text/plain
Authorization: Bearer ${DEFI_TOOLS_API_KEY}
```

### 5) Prediction market and project outlook

- API: `projectTool`
- Use when: user asks project odds, FDV expectations, short/long-term target prices, market sentiment for a project.
- Input:
  - `projectName` (required): project name, e.g. `berachain`
- Request:

```bash
GET https://defi-tools-proxy.saharaa.info/project/analysis?projectName=berachain
Headers:
accept: text/plain
Authorization: Bearer ${DEFI_TOOLS_API_KEY}
```

## Output Style for End Users

Use this response structure:

1. **What I queried**: API name and key parameters.
2. **Key findings**: 3-5 bullets with metrics.
3. **Interpretation**: trend/risk/opportunity in plain language.
4. **Actionable next step**: one concrete follow-up.
5. **Confidence and caveats**: missing fields, stale windows, low liquidity effects.

## Decision Heuristics

- If question is asset-specific, start with `tokenTool`.
- If question is yield-focused, start with `poolTool`.
- If question is chain ecosystem-focused, use `chainTool`.
- If question is protocol financial health-focused, use `protocolTool`.
- If question is project narrative or valuation-focused, use `projectTool`.

## Safety Rules

- Never invent prices, holders, APY, TVL, or tx outcomes.
- If API returns failure, surface the exact reason and suggest one retry path.
- Ask follow-up questions only for missing required parameters.
- Keep chain IDs explicit when ambiguity exists.
