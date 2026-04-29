---
name: polymarket-event-optimizer
description: "Advanced multi-event correlation, hedging, and arbitrage strategy for Polymarket."
version: "1.0.0"
author: "OKX Plugin Developer"
tags:
  - polymarket
  - strategy
  - arbitrage
  - hedging
---

# Polymarket Event Optimizer Strategy

## Overview

The Polymarket Event Optimizer is an advanced AI-driven strategy skill. It enables the AI agent to orchestrate multi-event correlation analysis, identify hedging/arbitrage opportunities across related prediction markets, and execute complex multi-leg trades. This strategy utilizes a custom Python binary tool (`optimizer-calc`) for precise mathematical modeling and the `polymarket-plugin` for market interactions and order execution.

## Pre-flight Checks

Before using this skill, ensure:

1. The `onchainos` CLI is installed and configured in your environment.
2. The `polymarket-plugin` is installed and the wallet is funded with USDC.e and POL on Polygon.
3. The `optimizer-calc` binary is installed via plugin-store (automatically handled).
4. You have read and accepted the risks detailed in the **Security Notices** section.
5. **Dry-Run Default**: Start by executing commands with the `--dry-run` parameter natively supported by your AI environment or simulate the execution logic before live trading.

## Strategy Parameters

**MANDATORY**: Every write operation (buy, sell, order, swap) executed via the dependent `polymarket-plugin` MUST include the following flag:
`--strategy-id polymarket-event-optimizer`

## Binary Tools (provided by this plugin)

### optimizer-calc
A high-performance Python calculator that fetches real-time market data from the Polymarket API to compute risk-free arbitrage spreads and multi-leg hedging costs.
**Parameters**: `--query <topic>` (string)
**Returns**: A JSON array of correlated markets, combined cost, and ROI percentage.

## Commands

### 1. analyze-events

Analyze and discover correlated markets to find arbitrage or hedging gaps using the binary tool.

**When to use**: When the user wants to find new trading opportunities, hedge an existing position, or calculate the probability spread between mutually exclusive events.
**Output**: A list of correlated market pairs, their current implied probabilities (prices), the calculated spread/gap, and a recommended action.
**Example**:
```bash
# 1. Run the mathematical calculation engine to discover gaps
optimizer-calc --query "Presidential Election 2024"

# 2. The AI reads the JSON output (e.g., combined_cost: 0.95, expected_payout: 1.00)
# 3. The AI presents the proposed trade sizes and expected yield to the user.
```

### 2. execute-hedge

Execute a multi-leg order across correlated markets to lock in the calculated spread.

**When to use**: After `analyze-events` identifies an opportunity and the user confirms execution.
**Output**: Confirmation of the executed orders, transaction hashes, and the new portfolio state.
**Example**:
```bash
# 1. Ask user for confirmation
# 2. Execute Leg 1 based on the output of optimizer-calc
polymarket-plugin buy --market "0x123...abc" --outcome YES --amount 100 --strategy-id polymarket-event-optimizer --confirm

# 3. Execute Leg 2 based on the output of optimizer-calc
polymarket-plugin buy --market "0x456...def" --outcome NO --amount 100 --strategy-id polymarket-event-optimizer --confirm
```

*Note: If the user requests a simulation, you must simulate the outputs and NOT run the execution commands.*

### 3. rebalance-portfolio

Monitor existing positions and adjust exposure based on shifting probabilities and dynamic thresholds.

**When to use**: Periodically, or when the user asks to "rebalance my Polymarket portfolio" or "check my hedges".
**Output**: Current P&L for the strategy, probability shifts since entry, and execution of any required adjustment trades.
**Example**:
```bash
# 1. Check current portfolio
polymarket-plugin portfolio

# 2. Re-run the optimizer-calc on held positions to detect if the spread has widened/narrowed beyond 10%
# 3. Execute rebalance trade if threshold is met:
polymarket-plugin sell --market "0x123...abc" --outcome YES --shares 50 --strategy-id polymarket-event-optimizer --confirm
```

## Risk Management (Advanced Tier)

As an `advanced` tier strategy, the AI MUST adhere to the following rules:

1. **Maximum Position Size**: Never allocate more than 20% of the total available USDC.e balance to a single market leg unless explicitly overridden by the user.
2. **Stop-Loss Enforcement**: If the combined value of a hedged position drops by more than 15% from the entry cost, the AI must proactively suggest liquidating the position.
3. **Slippage Protection**: Always ensure the execution price does not deviate significantly from the quoted price during the `analyze-events` phase.

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| Binary connection failed | The `optimizer-calc` script cannot reach the Polymarket API. | Fallback to simulation mode or check network connection. |
| `Insufficient balance` | Wallet lacks USDC.e or POL for gas. | Run `onchainos portfolio all-balances` and prompt the user to deposit funds. |
| `Market resolved` | The target prediction market has already settled. | Abort the trade and suggest `polymarket-plugin claim` if the user holds winning shares. |
| `Slippage exceeded` | High volatility during multi-leg execution. | Pause the strategy. Evaluate the filled legs and ask the user whether to close the filled leg or wait for better pricing on the remaining legs. |
| `Missing strategy-id` | AI forgot to append the required flag. | AI must immediately correct its command format to include `--strategy-id polymarket-event-optimizer`. |

## Security Notices

- **Financial Risk**: This plugin executes live financial transactions. The strategies (arbitrage, hedging) rely on market liquidity and order book depth, which can change rapidly.
- **Dry-Run Requirement**: Always propose a dry-run or simulated execution path to the user before committing real funds.
- **Approval Required**: The AI agent MUST request explicit user confirmation showing the exact amount, markets, and expected outcomes before executing the `execute-hedge` or `rebalance-portfolio` workflows.
