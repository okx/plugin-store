## Overview

Swap tokens on Raydium — Solana's largest AMM — with live quotes, multi-mint price checks, pool browsing, and a preview-before-execute flow using your onchainos wallet.

## Prerequisites
- onchainos agentic wallet connected
- Some SOL for gas plus the swap amount

## How it Works
1. **Check your wallet**: Get a personalised next step based on your balances and active positions. `raydium-plugin quickstart`
   - If `status: no_funds` — fund your Solana wallet with SOL or USDC first
   - If `status: needs_gas` — send at least 0.01 SOL to your wallet for transaction fees
   - If `status: ready_sol_only` — you have SOL; swap for USDC or other tokens
   - If `status: ready` — proceed to swap below
2. **Discover**: Browse and research before swapping.
   - 2.1 **Get a live quote**: See the expected output before committing — no gas. `raydium-plugin get-swap-quote --input-mint <input-mint> --output-mint <output-mint> --amount <amount>`
   - 2.2 **Check token prices**: Look up current prices for one or more token mints. `raydium-plugin get-token-price --mints <mint>`
   - 2.3 **Browse pools**: Find pools sorted by liquidity, volume, or APR. `raydium-plugin get-pool-list --sort-field liquidity --sort-type desc --page-size 5`
3. **Swap**:
   - 3.1 **Preview**: See the full transaction details before signing — no gas, no transaction. `raydium-plugin swap --input-mint <mint> --output-mint <mint> --amount <amount> --slippage-bps 50`
   - 3.2 **Execute**: Broadcast the transaction after confirming the preview. `raydium-plugin swap --input-mint <mint> --output-mint <mint> --amount <amount> --slippage-bps 50 --confirm`
   - 3.3 **Common mints**: SOL `So11111111111111111111111111111111111111112` · USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` · USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
