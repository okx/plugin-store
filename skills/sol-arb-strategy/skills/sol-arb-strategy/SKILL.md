---
name: sol-arb-strategy
description: "SOL arbitrage strategy between Raydium and Orca"
version: "1.0.0"
author: "yz06276"
tags:
  - solana
  - arbitrage
  - strategy
---

# sol-arb-strategy

## Overview

An automated arbitrage strategy that monitors SOL price differences between Raydium and Orca DEXs on Solana, executing trades when profitable spreads are detected.

## Pre-flight Checks

Before using this strategy, ensure:

1. The `onchainos` CLI is installed
2. `raydium-plugin` and `orca-plugin` are installed
3. A funded Solana wallet is configured

## Commands

### Run Strategy

```bash
python3 scripts/arb.py --pair SOL/USDC --threshold 0.5
```

**When to use**: When the user wants to run the SOL arbitrage strategy.
**Output**: Monitors prices and executes trades when spread exceeds threshold.

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Insufficient balance" | Not enough SOL/USDC | Fund wallet |
| "Spread too low" | No profitable opportunity | Wait or lower threshold |
