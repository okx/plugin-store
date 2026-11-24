## Overview

Add concentrated liquidity to Meteora DLMM pools on Solana — earning fees only on actively-traded price bins — with support for SOL-only, token-only, or two-sided deposits.

## Prerequisites
- onchainos agentic wallet connected
- Some SOL for transaction fees

## Quick Start
1. **Check your wallet**: Get a personalised next step based on your balances and active positions. `meteora-plugin quickstart`
   - If `status: no_funds` — fund your Solana wallet with SOL and optionally USDC first
   - If `status: needs_gas` — send at least 0.01 SOL to your wallet for transaction fees
   - If `status: ready_sol_only` — you have SOL only; add SOL-only liquidity or swap for USDC first
   - If `status: ready` — proceed to explore pools below
2. **Find pools**: Browse pools sorted by volume, TVL, or APR. `meteora-plugin get-pools --search-term SOL-USDC --sort-key volume --order-by desc`
3. **Get pool details**: See active bin price, fee tier, TVL, and bin step for a specific pool. `meteora-plugin get-pool-detail --address <pool>`
4. **Check existing positions**: List open positions with bin ranges and accrued fees. `meteora-plugin get-user-positions`
5. **Swap**:
   - 5.1 **Get a quote**: Check expected output before committing — no gas. `meteora-plugin get-swap-quote --from-token <input-mint> --to-token <output-mint> --amount <n>`
   - 5.2 **Execute the swap**: Swap against the DLMM pool. `meteora-plugin swap --from-token <input-mint> --to-token <output-mint> --amount <n> --confirm`
6. **Provide liquidity**:
   - 6.1 **Add liquidity**: Deposit into price bins to earn fees — omit one side for single-token deposit. `meteora-plugin add-liquidity --pool <address> --amount-x <amount-x> --amount-y <amount-y> --confirm`
   - 6.2 **Remove liquidity**: Withdraw your position and collect accrued fees. `meteora-plugin remove-liquidity --pool <address> --position <address> --confirm`
