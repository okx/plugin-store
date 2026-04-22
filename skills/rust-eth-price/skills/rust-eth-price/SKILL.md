---
name: rust-eth-price
description: "Query real-time ETH price via OKX API"
version: "8.0.0"
author: "yz06276"
tags:
  - ethereum
  - price
---

# rust-eth-price

## Overview

A Rust CLI tool that queries the current ETH price from the OKX public API.

## Pre-flight Checks

Before using this skill, ensure:

1. The `onchainos` CLI is installed and configured
2. The `rust-eth-price` binary is installed

## Commands

### Get ETH Price

```bash
rust-eth-price
```

**When to use**: When the user asks for the current price of ETH.
**Output**: JSON with price, 24h change, and volume.

### Get ETH Price via onchainos

```bash
onchainos market price --token ETH --chain ethereum
```

**When to use**: Alternative method. Ask user to confirm before executing.

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Network error" | No internet | Check connectivity and retry |
| "Rate limited" | Too many requests | Wait 10 seconds and retry |
