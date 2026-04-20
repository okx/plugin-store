**Overview**

Swap tokens on Raydium — Solana's largest AMM — with live quotes, multi-mint price checks, pool browsing, and a preview-before-execute flow using your onchainos wallet.

**Prerequisites**
- onchainos agentic wallet connected
- Some SOL for gas plus the swap amount

**How it Works**
1. **Discover**: Browse and research before swapping.
   - 1.1 **Get a live quote**: See the expected output before committing — no gas. `raydium-plugin get-swap-quote --input-mint <input-mint> --output-mint <output-mint> --amount <amount>`
   - 1.2 **Check token prices**: Look up current prices for one or more token mints. `raydium-plugin get-token-price --mints <mint>`
   - 1.3 **Browse pools**: Find pools sorted by liquidity, volume, or APR. `raydium-plugin get-pool-list --sort-field liquidity --sort-type desc --page-size 5`
2. **Swap**:
   - 2.1 **Preview**: See the full transaction details before signing — no gas, no transaction. `raydium-plugin swap --input-mint <mint> --output-mint <mint> --amount <amount> --slippage-bps 50`
   - 2.2 **Execute**: Broadcast the transaction after confirming the preview. `raydium-plugin swap --input-mint <mint> --output-mint <mint> --amount <amount> --slippage-bps 50 --confirm`
   - 2.3 **Common mints**: SOL `So11111111111111111111111111111111111111112` · USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` · USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
