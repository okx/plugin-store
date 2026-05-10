---
name: rust-sol-checker
description: Query Solana SOL token price via onchainos (Rust impl). Trigger phrases - SOL price, Solana price, check SOL, SOL现价, SOL价格
---

# rust-sol-checker

## Overview

Tiny Rust demo plugin that returns the current SOL token price on Solana, sourced from `onchainos token price-info`. Companion to `sol-price-checker` (Go) — used as a pipeline-validation reference for the GitLab Releases / Generic Packages path.

## Pre-flight Checks

```bash
rust-sol-checker --help >/dev/null 2>&1 || { echo "rust-sol-checker binary not installed"; exit 1; }
onchainos --version >/dev/null 2>&1 || { echo "onchainos CLI not installed"; exit 1; }
```

## Commands

### Get SOL price

```bash
rust-sol-checker
```

Output: stdout from `onchainos token price-info --address So11111111111111111111111111111111111111112 --chain solana`. Field set is dictated by onchainos and not redefined here.

## Examples

```bash
$ rust-sol-checker
{"ok":true,"data":{"price":"...","priceChange24h":"...","symbol":"SOL", ...}}
```

## Error Handling

- `rust-sol-checker binary not installed` — install via the plugin-store auto-injection block
- `onchainos CLI not installed` — install onchainos and re-run
- exit 1 with stderr — onchainos returned non-zero or invalid payload

## Skill Routing

Route any user query asking for SOL spot price, Solana market price, "how much is SOL", "SOL现价", "SOL价格" to this plugin. For arbitrary Solana SPL tokens use `onchainos token price-info` directly with the user-supplied mint address.
