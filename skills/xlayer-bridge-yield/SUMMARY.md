## Overview

xlayer-bridge-yield is a DeFi automation plugin that bridges stablecoins to X Layer (OKX's L2) and optimizes yield farming positions with built-in risk management.

Core operations:

- Compare cross-chain bridge routes for cost and speed optimization
- Scan and rank X Layer yield pools by APY, TVL, and risk score
- Execute the full bridge-to-farm pipeline with pre-execution safety checks
- Monitor active positions with depeg alerts and rebalance suggestions

Tags: `xlayer` `bridge` `yield` `stablecoin` `defi` `defi-protocol`

## Prerequisites

- No IP restrictions
- Supported source chains: Ethereum (1), BSC (56), Polygon (137), Arbitrum (42161), Solana
- Supported destination: X Layer (chain ID 196)
- Supported tokens: USDC, USDT, DAI
- onchainos CLI installed and authenticated (`npx skills add okx/onchainos-skills`)
- Python 3.8+ installed (for helper scripts)
- A funded wallet on the source chain with sufficient tokens and gas

## Quick Start

1. **Scan bridge routes**: Ask the agent "find the cheapest bridge for 100 USDC from Ethereum to X Layer". The agent compares available cross-chain routes and presents a ranked table with fees, estimated time, and slippage for each option.

2. **Scan yield pools**: Ask "show the best yield pools on X Layer for USDT". The agent scans DeFi protocols on X Layer, runs security checks on each pool token, and ranks results by APY adjusted for risk. Pools with critical security issues or low TVL are automatically filtered out.

3. **Bridge and farm**: Ask "bridge 200 USDC to X Layer and farm the best yield". The agent combines both scans, presents a complete plan (bridge cost + target pool + expected yield), and waits for your explicit confirmation before executing any transaction. Default mode is dry-run — no real transactions occur unless you say "confirm" or "execute".

4. **Monitor positions**: Ask "check my X Layer farming positions". The agent shows current value, profit/loss, and any alerts (depeg risk, APY changes, stop-loss triggers). If a better pool is available, it suggests rebalancing.
