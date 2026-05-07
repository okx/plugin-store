## Overview

Swap tokens on Orca Whirlpools — Solana's leading concentrated liquidity DEX — with auto-routing to the best pool for your token pair.

## Prerequisites
- onchainos agentic wallet connected
- Some SOL for transaction fees

## How it Works
1. **Check your wallet**: Get a personalised next step based on your balances. `orca-plugin quickstart`
   - If `status: no_funds` or `needs_gas` — fund your Solana wallet with SOL first
   - If `status: ready` or `ready_sol_only` — proceed below
2. **Discover pools**: Find all Whirlpool pools for a token pair with TVL, fee tier, and current price. `orca-plugin get-pools --token-a <mint> --token-b <mint>`
3. **Get a swap quote**: Check expected output and best pool — no gas. `orca-plugin get-quote --from-token <mint> --to-token <mint> --amount <n>`
4. **Execute the swap**: Swap tokens at the quoted rate — default slippage 0.5%. `orca-plugin swap --from-token <mint> --to-token <mint> --amount <n> --confirm`
   - 4.1 **Non-SOL tokens**: The input token must be in your wallet — SPL token account is created automatically if needed.
   - 4.2 **Common mints**: SOL `11111111111111111111111111111111` · USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` · USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
