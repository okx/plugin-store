## Overview

Low-slippage swaps and liquidity provision on Curve Finance — optimized for pegged assets (stablecoins, LSTs, wrapped BTC) across Ethereum, Arbitrum, Base, Polygon, and BSC.

## Prerequisites
- onchainos agentic wallet connected
- Some tokens on a supported chain — Ethereum (default), Arbitrum, Base, Polygon, or BSC

## Quick Start
1. **Check your wallet**: Get a personalised next step based on your balances on the active chain. `curve-plugin quickstart`
   - If `status: no_funds` or `needs_gas` — fund your wallet with the native gas token first
   - If `status: needs_funds` — you have gas but no stablecoins; transfer USDC or USDT to your wallet
   - If `status: ready` — proceed below
2. **Find pools**: Browse available Curve pools with TVL, APY, and fee data. `curve-plugin get-pools`
3. **Get pool details**: See reserves, current APY, and virtual price for a specific pool. `curve-plugin get-pool-info --pool <address>`
4. **Swap**:
   - 4.1 **Get a quote**: Check the expected output before committing — no gas. `curve-plugin quote --token-in <USDC> --token-out <DAI> --amount <amount>`
   - 4.2 **Execute the swap**: Send input token and receive output in one transaction. `curve-plugin swap --token-in <USDC> --token-out <DAI> --amount <amount> --confirm`
5. **Provide liquidity**:
   - 5.1 **Add liquidity**: Deposit one or more pool tokens to receive LP tokens — amounts are comma-separated in pool coin order (e.g. `0,500,500` for a 3-token pool). `curve-plugin add-liquidity --pool <address> --amounts <amount>,... --confirm`
   - 5.2 **Check LP balance**: View your current LP token holdings for a pool. `curve-plugin get-balances --pool <address>`
   - 5.3 **Remove liquidity**: Burn LP tokens to withdraw the underlying pool assets. `curve-plugin remove-liquidity --pool <address> --lp-amount <lp-tokens> --confirm`
