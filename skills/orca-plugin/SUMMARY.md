**Overview**

Swap tokens on Orca Whirlpools — Solana's leading concentrated liquidity DEX — with auto-routing to the best pool for your token pair.

**Prerequisites**
- onchainos agentic wallet connected with a Solana wallet (chain 501)
- Some SOL for transaction fees

**How it Works**
1. **Discover pools**: Find all Whirlpool pools for a token pair with TVL, fee tier, and current price. `orca-plugin get-pools --token-a <mint> --token-b <mint>`
2. **Get a swap quote**: Check expected output and best pool — no gas. `orca-plugin get-quote --input-mint <mint> --output-mint <mint> --amount <n>`
3. **Execute the swap**: Swap tokens at the quoted rate — default slippage 0.5%. `orca-plugin swap --input-mint <mint> --output-mint <mint> --amount <n> --confirm`
   - 3.1 **Non-SOL tokens**: The input token must be in your wallet — SPL token account is created automatically if needed.
   - 3.2 **Common mints**: SOL `So11111111111111111111111111111111111111112` · USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` · USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`
