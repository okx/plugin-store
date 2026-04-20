**Overview**

Low-slippage swaps and liquidity provision on Curve Finance — optimized for pegged assets (stablecoins, LSTs, wrapped BTC) across Ethereum, Arbitrum, Base, Polygon, and BSC.

**Prerequisites**
- onchainos agentic wallet connected
- Some tokens on a supported chain — Ethereum (default), Arbitrum, Base, Polygon, or BSC

**How it Works**
1. **Find pools**: Browse available Curve pools with TVL, APY, and fee data. `curve-plugin get-pools`
2. **Get pool details**: See reserves, current APY, and virtual price for a specific pool. `curve-plugin get-pool-info --pool <address>`
3. **Swap**:
   - 3.1 **Get a quote**: Check the expected output before committing — no gas. `curve-plugin quote --input-mint <USDC> --output-mint <DAI> --amount 1000`
   - 3.2 **Execute the swap**: Send input token and receive output in one transaction. `curve-plugin swap --input-mint <USDC> --output-mint <DAI> --amount 1000 --slippage 0.5 --confirm`
4. **Provide liquidity**:
   - 4.1 **Add liquidity**: Deposit one or more pool tokens to receive LP tokens. `curve-plugin add-liquidity --pool <address> --amounts <token>:<amount>,... --confirm`
   - 4.2 **Check LP balance**: View your current LP token holdings for a pool. `curve-plugin get-balances --pool <address>`
   - 4.3 **Remove liquidity**: Burn LP tokens to withdraw the underlying pool assets. `curve-plugin remove-liquidity --pool <address> --amount <lp-tokens> --confirm`
