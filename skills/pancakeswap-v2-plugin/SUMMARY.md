## Overview

Swap tokens and manage liquidity on PancakeSwap V2's constant-product AMM (0.25% fee) across BSC, Base, and Arbitrum — LP tokens are standard ERC-20 and composable with other DeFi protocols.

## Prerequisites
- onchainos agentic wallet connected
- Some tokens on a supported chain — BSC (default), Base, or Arbitrum

## Quick Start
1. **Check your wallet**: Get a personalised next step based on your balances on the active chain. `pancakeswap-v2-plugin quickstart`
   - If `status: no_funds` or `needs_gas` — fund your wallet with the native gas token first
   - If `status: needs_funds` — you have gas but no tokens; transfer tokens to your wallet or swap the native token
   - If `status: ready` — proceed below
2. **Swap**:
   - 2.1 **Get a quote**: Check the expected output before committing — no gas. `pancakeswap-v2-plugin quote --token-in USDT --token-out CAKE --amount-in <amount>`
   - 2.2 **Execute the swap**: Send input token and receive output — ERC-20 approval fires automatically if needed. `pancakeswap-v2-plugin swap --token-in USDT --token-out CAKE --amount-in <amount> --confirm`
3. **Provide liquidity**:
   - 3.1 **Look up a pair**: Find the LP contract address for a token pair. `pancakeswap-v2-plugin get-pair --token-a CAKE --token-b BNB`
   - 3.2 **Check reserves**: See the current token balances and implied price in a pair. `pancakeswap-v2-plugin get-reserves --token-a CAKE --token-b BNB`
   - 3.3 **Add liquidity**: Deposit both tokens of the pair to receive LP tokens. `pancakeswap-v2-plugin add-liquidity --token-a CAKE --token-b BNB --amount-a <amount-a> --amount-b <amount-b> --confirm`
   - 3.4 **Check LP balance**: View your LP token holdings for a pair. `pancakeswap-v2-plugin lp-balance --token-a CAKE --token-b BNB`
   - 3.5 **Remove liquidity**: Burn LP tokens to withdraw your proportional share of the pool. `pancakeswap-v2-plugin remove-liquidity --token-a CAKE --token-b BNB --liquidity <amount> --confirm`
