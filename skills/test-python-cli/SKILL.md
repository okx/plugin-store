---
name: test-python-cli
description: "E2E test - Python CLI plugin"
version: "1.0.0"
author: "E2E Test"
tags: [test, python, onchainos]
---

# Test Python CLI

## Overview
E2E test plugin with Python script and OnchainOS integration.

## Pre-flight Checks
1. Install onchainos CLI: `curl -sSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh`
2. Ensure Python 3 is installed

## Commands

### Query ETH Price via Python
```bash
python3 scripts/query_price.py --query eth-price
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
| Script not found | Plugin not installed | Install via plugin-store |
| Command not found | onchainos not installed | Run pre-flight install |
