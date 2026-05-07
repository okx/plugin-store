## Overview

Supply assets and borrow against collateral on Aave V3 across Ethereum, Base, Polygon, and Arbitrum — with real-time Health Factor tracking to prevent liquidation.

## Prerequisites
- onchainos agentic wallet connected
- Some tokens on a supported chain — Ethereum, Base (default), Polygon, or Arbitrum

## How it Works
1. **Check your wallet**: Get a personalised next step based on your balances and active positions. `aave-v3-plugin quickstart`
   - If `status: no_funds` or `needs_gas` — fund your wallet first
   - If `status: needs_funds` — you have gas but no assets to supply; add USDC or WETH to your wallet
   - If `status: ready` — proceed to supply below
   - If `status: active` — you already have a position; monitor your Health Factor
2. **Supply**:
   - 2.1 **Check available markets**: Browse assets with supply APY, borrow rate, and utilization. `aave-v3-plugin reserves`
   - 2.2 **Supply assets**: Deposit tokens to earn yield — ERC-20 approval fires automatically. `aave-v3-plugin supply --asset USDC --amount <amount> --confirm`
   - 2.3 **Monitor your position**: View total collateral, debt, borrow power, and Health Factor. `aave-v3-plugin positions`
3. **Borrow** (requires collateral supplied first; Health Factor must stay above 1.0):
   - 3.1 **Borrow**: Draw against your supplied collateral at the variable rate. `aave-v3-plugin borrow --asset WETH --amount <amount> --confirm`
   - 3.2 **Repay**: Return borrowed assets and free up collateral — use `--all` to repay in full. `aave-v3-plugin repay --asset WETH --amount <amount> --confirm`
   - 3.3 **Withdraw collateral**: Reclaim supplied assets — only possible while Health Factor stays above 1.0. `aave-v3-plugin withdraw --asset USDC --amount <amount> --confirm`
