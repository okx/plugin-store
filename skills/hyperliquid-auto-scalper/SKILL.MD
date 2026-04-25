---
name: hyperliquid-auto-scalper
description: "Intelligent multi-coin scalping strategy with RSI mean-reversion, funding rate filter and micro-batch execution on Hyperliquid"
version: "1.0.0"
author: "Nolan Rothschild"
tags:
  - hyperliquid
  - scalping
  - perps
  - mean-reversion
  - rsi
  - multi-coin
  - risk-management
---

# Hyperliquid Auto Scalper

## Overview
Hyperliquid Auto Scalper is an intelligent, high-frequency scalping strategy for Hyperliquid perpetuals.  
The skill automatically rotates between BTC, ETH, and SOL, uses RSI for mean-reversion signals, applies a strict low funding rate filter, and executes micro-batches ( Apply TWAP to get the best price, 10 small orders per signal).

**Primary Goal**: Maximize Trading Volume, Transaction Count, and Active Addresses for the Plugin Store Developer Challenge Season 1 while keeping risk low.

## Prerequisites
1. Hyperliquid Plugin is installed and registered (`hyperliquid register`).
2. Agentic Wallet has sufficient USDC margin on Hyperliquid (recommended ≥ $100).
3. Skill has been built (`npm run build`).
4. Run `hyperliquid quickstart` before first use.

## When to Use This Skill
Use **Hyperliquid Auto Scalper** when you want to:
- Run automated multi-coin scalping
- Generate frequent small trades in ranging markets
- Participate in the Plugin Store Challenge with high activity
- Trade with low risk (2x leverage, tiny position sizes)

**Best Market Conditions**: Sideways/range-bound markets with low funding rates.

## How It Works
When the user says a command containing “run”, the agent will:
1. Rotate through BTC-PERP, ETH-PERP, SOL-PERP.
2. Check if absolute funding rate < 0.01.
3. Calculate RSI from recent klines.
4. If RSI < 30 → Buy micro-batch  
   If RSI > 70 → Sell micro-batch
5. Creates 10 micro orders per signal  - TWAP (Time-Averaged Price) is an algorithmic trading strategy that breaks down a large buy or sell order into multiple smaller orders. Apply TWAP to get the best price.
6. All orders are executed with `strategyId: "hyperliquid-auto-scalper"`.

### Key Commands

**Run the Strategy**
```bash
# Just tell the agent:
Run hyperliquid auto scalper
or
Run scalper