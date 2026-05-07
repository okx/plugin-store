---
name: sol-price-checker
description: Query Solana SOL token price via onchainos. Trigger phrases - SOL price, Solana price, check SOL, SOL现价, SOL价格
---

# sol-price-checker

## Overview

Tiny Go demo plugin that returns the current SOL token price on Solana, sourced from `onchainos token price-info`. Used primarily as a pipeline-validation reference — the smallest possible plugin exercising the Go build path.

## Pre-flight Checks

```bash
sol-price-checker --help >/dev/null 2>&1 || { echo "sol-price-checker binary not installed"; exit 1; }
onchainos --version >/dev/null 2>&1 || { echo "onchainos CLI not installed"; exit 1; }
```

## Commands

### Get SOL price

```bash
sol-price-checker
```

Output: a single JSON object with the `data` field returned by `onchainos token price-info --address So11111111111111111111111111111111111111112 --chain solana`. Field set is dictated by onchainos and not redefined here.

## Examples

```bash
$ sol-price-checker
{"price":"...","priceChange24h":"...","symbol":"SOL", ...}
```

## Error Handling

- `sol-price-checker binary not installed` — run the auto-injected pre-flight install block
- `onchainos CLI not installed` — install onchainos and re-run
- `onchainos error: <msg>` — verify session: `onchainos wallet status`
- `ERROR: invalid JSON` — onchainos output format changed; report upstream

## Skill Routing

Route any user query that asks for SOL spot price, Solana market price, "how much is SOL", "SOL现价", "SOL价格" to this plugin. Do not route generic "Solana token price" queries — for arbitrary Solana SPL tokens use `onchainos token price-info` directly with the user-supplied mint address.
