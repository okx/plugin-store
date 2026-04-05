---
name: test-pure-skill
description: "E2E test - pure skill plugin"
version: "1.0.0"
author: "E2E Test"
tags: [test, onchainos]
---

# Test Pure Skill

## Overview
E2E test plugin for verifying pure skill submission flow.

## Pre-flight Checks
1. Install onchainos CLI: `curl -sSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh`

## Commands

### Query ETH Price
```bash
onchainos token price ETH
```
**When to use:** When user asks about ETH price or market data.
**Output:** Current ETH price in USD.

## Error Handling
| Error | Cause | Resolution |
|-------|-------|------------|
| Command not found | onchainos not installed | Run pre-flight install |
