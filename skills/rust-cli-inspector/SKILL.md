---
name: rust-cli-inspector
description: "Rust CLI querying ETH price via OnchainOS"
version: "1.0.0"
author: "OKX"
tags: [rust, onchainos]
---

# Rust CLI Inspector

## Overview
Queries ETH price via OnchainOS token price-info.

## Pre-flight Checks
1. Install onchainos CLI
2. Ensure rust-cli-inspector binary is installed

## Commands

### Query ETH Price
`rust-cli-inspector --query eth-price`

**When to use:** When user asks about ETH price.

### Direct OnchainOS Query
`onchainos token price-info --address 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --chain ethereum`

## Error Handling
| Error | Cause | Resolution |
|-------|-------|------------|
| Binary not found | CLI not installed | Run pre-flight |
| Command not found | onchainos not installed | Install onchainos |
