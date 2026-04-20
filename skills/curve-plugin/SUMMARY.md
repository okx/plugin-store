**Overview**

Low-slippage swaps and liquidity provision on Curve Finance — optimized for pegged assets (stablecoins, LSTs, wrapped BTC) across Ethereum, Arbitrum, Base, Polygon, and BSC.

**Prerequisites**
- onchainos agentic wallet connected
- Wallet with funds on Ethereum (1, default), Arbitrum (42161), Base (8453), Polygon (137), or BSC (56)
- For swaps: the input token in your wallet; for `add-liquidity`: one or more pool tokens
- Pass `--chain <id>` to switch chains

**How it Works**
1. **Find pools**: Browse available Curve pools with TVL, APY, and fee data. `curve-plugin get-pools`
2. **Get pool details**: See reserves, current APY, and virtual price for a specific pool. `curve-plugin get-pool-info --pool <address>`
3. **Get a swap quote**: Check the expected output before committing — no gas. `curve-plugin quote --input-mint <USDC> --output-mint <DAI> --amount 1000`
4. **Execute the swap**: Send input token and receive output in one transaction. `curve-plugin swap --input-mint <USDC> --output-mint <DAI> --amount 1000 --slippage 0.5 --confirm`
5. **Add liquidity**: Deposit one or more pool tokens to receive LP tokens. `curve-plugin add-liquidity --pool <address> --amounts <token>:<amount>,... --confirm`
6. **Check LP balance**: View your current LP token holdings for a pool. `curve-plugin get-balances --pool <address>`
7. **Remove liquidity**: Burn LP tokens to withdraw the underlying pool assets. `curve-plugin remove-liquidity --pool <address> --amount <lp-tokens> --confirm`
