---
name: polymarket-optimizer
description: "Advanced multi-event correlation, hedging, and arbitrage strategy for Polymarket. Default dry-run mode, configurable stop-loss, and max amount limits included."
version: "1.0.0"
author: "ZhengTZ"
tags:
  - polymarket
  - strategy
  - arbitrage
  - hedging
---

# Polymarket Event Optimizer Strategy

> **This strategy is a real trading bot. Make sure you understand the risks before use. It is recommended to test in Dry-Run Mode first.**

---

## Disclaimer

**This strategy script, parameter configuration, and all related documentation are for educational research and technical reference only, and do not constitute any form of investment advice, trading guidance, or financial recommendation.**

1. **Extreme Risk Warning**: Polymarket prediction markets can be highly volatile. Market probabilities can shift rapidly based on real-world events. You may lose your entire invested capital.
2. **Parameters for Reference Only**: All default parameters in this strategy are set based on general scenarios and **are not guaranteed to be suitable for any specific market environment**.
3. **No Guarantee of Profit**: Past performance does not represent future results. Arbitrage gaps may close before execution completes, resulting in directional exposure rather than risk-free profit.
4. **Execution Risks**: On-chain transactions are irreversible. Network congestion, slippage, and API latency may cause transaction failures or partial fills (leg risk).
5. **Assume All Responsibility**: This strategy is provided "AS-IS" without any express or implied warranties. All trading decisions made using this strategy and their consequences are the sole responsibility of the user. The strategy author, developers, distributors, and their affiliates are not liable for any direct, indirect, incidental, or special losses.

**Recommendation**: For first-time use, please ensure the default **Dry-Run Mode** is enabled to fully familiarize yourself with the strategy logic before considering live trading.

---

## Overview

The Polymarket Event Optimizer is an advanced AI-driven strategy skill. It enables the AI agent to orchestrate multi-event correlation analysis, identify hedging/arbitrage opportunities across related prediction markets, and execute complex multi-leg trades. This strategy utilizes a custom Python binary tool (`optimizer-calc`) for precise mathematical modeling and the `polymarket-plugin` for market interactions and order execution.

## Pre-flight Checks

Before using this skill, ensure:

1. The `onchainos` CLI is installed and configured in your environment.
2. The `polymarket-plugin` is installed and the wallet is funded with USDC.e and POL on Polygon.
3. The `optimizer-calc` binary is installed via plugin-store (automatically handled).
4. You have read and accepted the risks detailed in the **Security Notices** section.
5. **Dry-Run Default**: This plugin defaults to Dry-Run / Simulated Trading mode. Start by executing commands with the `--dry-run` parameter or rely on the native simulation. The AI MUST require explicit user opt-in to switch to live trading.

## Strategy Parameters

**MANDATORY**: Every write operation (buy, sell, order, swap) executed via the dependent `polymarket-plugin` MUST include the following flag:
`--strategy-id polymarket-optimizer`

## Configurable Risk Management (Advanced Tier)

As an `advanced` tier strategy, this plugin features strict, user-configurable risk guardrails. The AI MUST adhere to these rules and present them to the user for configuration before any live trading:

1. **Maximum Amount Limits**:
   - **Single Trade Limit**: Configurable maximum amount per leg (Default: 50 USDC).
   - **Session Limit**: Configurable maximum total exposure per AI session (Default: 200 USDC).
   - The AI must ask the user to confirm or adjust these limits before executing trades. Never exceed the configured caps.

2. **Stop-Loss Enforcement**: 
   - **Maximum Drawdown Threshold**: Configurable maximum loss threshold for a hedged position (Default: 15%).
   - If the combined value of a hedged position drops by more than the configured percentage from the entry cost, the AI must proactively alert the user and suggest liquidating the position.

3. **Slippage Protection**: Always ensure the execution price does not deviate significantly from the quoted price during the `analyze-events` phase.

4. **Two-Reviewer Rule**: Note for contributors: Any modifications to this advanced plugin require approval from at least two repository maintainers.

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

# 2. The AI reads the JSON output
# 3. The AI presents the proposed trade sizes and expected yield to the user.
```

### 2. execute-hedge

Execute a multi-leg order across correlated markets to lock in the calculated spread.

**When to use**: After `analyze-events` identifies an opportunity and the user confirms execution limits.
**Output**: Confirmation of the executed orders, transaction hashes, and the new portfolio state.
**Example**:
```bash
# 1. Ask user for confirmation and verify against configurable Session/Trade limits
# 2. Execute Leg 1
polymarket-plugin buy --market "0x123...abc" --outcome YES --amount 50 --strategy-id polymarket-optimizer --confirm

# 3. Execute Leg 2
polymarket-plugin buy --market "0x456...def" --outcome NO --amount 50 --strategy-id polymarket-optimizer --confirm
```

*Note: In the default Dry-Run mode, the AI must simulate the outputs and NOT run the execution commands.*

### 3. rebalance-portfolio

Monitor existing positions and adjust exposure based on shifting probabilities and dynamic thresholds.

**When to use**: Periodically, or when the user asks to "rebalance my Polymarket portfolio" or "check my hedges".
**Output**: Current P&L for the strategy, probability shifts since entry, and execution of any required adjustment trades.
**Example**:
```bash
# 1. Check current portfolio
polymarket-plugin portfolio

# 2. Check positions against the configurable Stop-Loss Threshold
# 3. Execute rebalance trade if threshold is met:
polymarket-plugin sell --market "0x123...abc" --outcome YES --shares 50 --strategy-id polymarket-optimizer --confirm
```

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| Binary connection failed | The `optimizer-calc` script cannot reach the Polymarket API. | Fallback to simulation mode or check network connection. |
| `Insufficient balance` | Wallet lacks USDC.e or POL for gas. | Run `onchainos portfolio all-balances` and prompt the user to deposit funds. |
| `Market resolved` | The target prediction market has already settled. | Abort the trade and suggest `polymarket-plugin claim`. |
| `Slippage exceeded` | High volatility during multi-leg execution. | Pause the strategy. Evaluate the filled legs and ask the user whether to close the filled leg or wait. |
| `Missing strategy-id` | AI forgot to append the required flag. | AI must immediately correct its command format. |

## Security Notices

- **Financial Risk**: This plugin executes live financial transactions. The strategies rely on market liquidity and order book depth, which can change rapidly.
- **Approval Required**: The AI agent MUST request explicit user confirmation showing the exact amount, markets, and expected outcomes before executing any live trades, ensuring they are within the user's configured single-trade and session limits.
