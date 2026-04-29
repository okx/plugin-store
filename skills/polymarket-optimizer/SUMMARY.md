# polymarket-optimizer

## Overview

The Polymarket Event Optimizer is an advanced AI strategy skill that orchestrates multi-event hedging, arbitrage, and dynamic portfolio rebalancing on Polymarket.

Core operations:

- Scan and analyze correlated prediction markets for probability gaps
- Execute multi-leg hedging and risk-free arbitrage trades
- Dynamically rebalance portfolios based on real-time probability shifts
- Enforce strict risk management, position sizing, and stop-loss rules

Tags: `strategy` `polymarket` `arbitrage` `hedging` `prediction-market`

## Prerequisites

- US users are restricted from trading on Polymarket
- Supported chain: Polygon (MATIC)
- Supported tokens: USDC.e for trading, POL for gas fees
- `onchainos` CLI installed and authenticated
- `polymarket-plugin` installed
- A funded wallet with sufficient USDC.e and POL

## Quick Start

1. **Analyze Opportunities**
   Ask the Agent to scan for arbitrage or hedging opportunities (e.g., "Find arbitrage gaps in the 2024 Presidential Election markets"). The Agent will calculate the probability spread across correlated outcomes.

2. **Simulate Strategy (Dry-Run)**
   Before risking capital, ask the Agent to simulate the execution. It will outline the exact trade sizes, expected yield, and maximum exposure without executing live transactions.

3. **Execute Hedged Trades**
   Once confirmed, instruct the Agent to execute the strategy. It will use the underlying `polymarket-plugin` to place multi-leg orders (e.g., buying YES on Candidate A and YES on Candidate B) to lock in the calculated spread. All trades are automatically tracked under this strategy.

4. **Dynamic Rebalancing**
   Periodically ask the Agent to "rebalance my Polymarket portfolio". It will evaluate your current hedges against live market prices and automatically take profit or cut losses according to the strategy's risk parameters.
