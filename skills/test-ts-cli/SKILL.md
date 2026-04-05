---
name: test-ts-cli
description: "E2E test - TypeScript CLI plugin"
version: "1.0.0"
author: "E2E Test"
tags: [test, typescript, onchainos]
---

# Test TypeScript CLI

## Overview
E2E test plugin with TypeScript CLI binary and OnchainOS integration.

## Pre-flight Checks
1. Install onchainos CLI: `curl -sSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh`
2. Ensure test-ts-cli binary is installed

## Commands

### Query ETH Price via CLI
```bash
test-ts-cli --query eth-price
```
**When to use:** When user asks about ETH price.
**Output:** Calls `onchainos token price-info --address 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --chain ethereum` and formats the result.

### Direct OnchainOS Query
```bash
onchainos token price-info --address 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --chain ethereum
```

## Error Handling
| Error | Cause | Resolution |
|-------|-------|------------|
| Binary not found | CLI not installed | Install via plugin-store |
| Command not found | onchainos not installed | Run pre-flight install |
