---
name: rust-sol-price
description: "Query real-time SOL price via OKX API"
version: "9.0.0"
author: "yz06276"
tags:
  - solana
  - price
---

# rust-sol-price

## Overview

A Rust CLI tool that queries the current SOL price from the OKX public API.

## Pre-flight Checks

Before using this skill, ensure:

1. The `onchainos` CLI is installed and configured
2. The `rust-sol-price` binary is installed

## Commands

### Get SOL Price

```bash
rust-sol-price
```

**When to use**: When the user asks for the current price of SOL.
**Output**: JSON with price, 24h change, and volume.

### Get SOL Price via onchainos

```bash
onchainos market price --token SOL --chain solana
```

**When to use**: Alternative method. Ask user to confirm before executing.

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Network error" | No internet | Check connectivity and retry |
| "Rate limited" | Too many requests | Wait 10 seconds and retry |
