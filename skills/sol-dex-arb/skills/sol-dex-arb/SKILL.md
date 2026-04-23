---
name: sol-dex-arb
description: "SOL DEX arbitrage strategy"
version: "1.0.0"
author: "yz06276"
tags: [solana, strategy]
---
# sol-dex-arb
## Overview
Arbitrage strategy exploiting SOL price differences on Raydium.
## Pre-flight Checks
1. raydium-plugin installed
2. Funded Solana wallet
## Commands
### Run
```bash
python3 scripts/arb.py
```
**When to use**: When user wants to run SOL arbitrage.
## Error Handling
| Error | Cause | Resolution |
|-------|-------|------------|
| "Spread too low" | No opportunity | Wait |
