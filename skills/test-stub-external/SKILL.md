---
name: test-stub-external
description: "E2E test - stub plugin from external repo"
version: "1.0.0"
author: "E2E Test"
tags: [test, stub, onchainos]
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Install onchainos CLI + Skills (auto-injected)

```bash
# 1. Install onchainos CLI
onchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh

# 2. Install onchainos skills (enables AI agent to use onchainos commands)
npx skills add okx/onchainos-skills --yes --global

# 3. Install plugin-store skills (enables plugin discovery and management)
npx skills add okx/plugin-store --skill plugin-store --yes --global
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/test-stub-external"
if [ ! -f "$REPORT_FLAG" ]; then
  mkdir -p "$HOME/.plugin-store/reported"
  # Generate device ID
  DEV_RAW="$(hostname):$(uname -s):$(uname -m):$HOME"
  DEV_ID=$(echo -n "$DEV_RAW" | shasum -a 256 | head -c 32)
  # Report to Vercel stats
  curl -s -X POST "https://plugin-store-dun.vercel.app/install" \
    -H "Content-Type: application/json" \
    -d '{"name":"test-stub-external","version":"1.0.0"}' >/dev/null 2>&1 || true
  # Report to OKX API
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"test-stub-external","divId":"'"$DEV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


# Test Stub External

## Overview
E2E test plugin loaded from an external repository via Mode B (repo + commit pointer).

## Pre-flight Checks
1. Install onchainos CLI: `curl -sSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh`

## Commands

### Query ETH Price
```bash
onchainos token price-info --address 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --chain ethereum
```
**When to use:** When user asks about ETH price.
**Output:** Current ETH price in USD.
