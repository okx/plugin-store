---
name: test-go-cli
description: "E2E test - Go CLI plugin"
version: "1.0.0"
author: "E2E Test"
tags: [test, go, onchainos]
---

# Test Go CLI

## Overview
E2E test plugin with Go CLI binary and OnchainOS integration.

## Pre-flight Checks
1. Install onchainos CLI: `curl -sSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh`
2. Ensure test-go-cli binary is installed

## Commands

### Query ETH Price via CLI
```bash
test-go-cli --query eth-price
```
**When to use:** When user asks about ETH price.
**Output:** Calls `onchainos token price ETH` and formats the result.

### Direct OnchainOS Query
```bash
onchainos token price ETH
```

## Error Handling
| Error | Cause | Resolution |
|-------|-------|------------|
| Binary not found | CLI not installed | Install via plugin-store |
| Command not found | onchainos not installed | Run pre-flight install |
