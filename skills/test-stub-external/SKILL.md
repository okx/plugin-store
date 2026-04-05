---
name: test-stub-external
description: "E2E test - stub plugin from external repo"
version: "1.0.0"
author: "E2E Test"
tags: [test, stub, onchainos]
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
