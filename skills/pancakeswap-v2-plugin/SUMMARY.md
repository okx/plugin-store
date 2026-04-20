**Overview**

Swap tokens and manage liquidity on PancakeSwap V2's constant-product AMM (0.25% fee) across BSC, Base, and Arbitrum — LP tokens are standard ERC-20 and composable with other DeFi protocols.

**Prerequisites**
- onchainos agentic wallet connected
- Wallet with funds on BSC (56, default), Base (8453), or Arbitrum One (42161)
- For swaps: the input token in your wallet; for `add-liquidity`: both tokens of the pair
- Pass `--chain <id>` to switch chains

**How it Works**
1. **Get a swap quote**: Check the expected output before committing — no gas. `pancakeswap-v2-plugin quote --token-in USDT --token-out CAKE --amount-in 100`
2. **Execute the swap**: Send input token and receive output — ERC-20 approval fires automatically if needed. `pancakeswap-v2-plugin swap --token-in USDT --token-out CAKE --amount-in 100 --confirm`
3. **Look up a pair**: Find the LP contract address for a token pair. `pancakeswap-v2-plugin get-pair --token-a CAKE --token-b BNB`
4. **Check reserves**: See the current token balances and implied price in a pair. `pancakeswap-v2-plugin get-reserves --pair <address>`
5. **Add liquidity**: Deposit both tokens to receive LP tokens representing your share of the pool. `pancakeswap-v2-plugin add-liquidity --token-a CAKE --token-b BNB --amount-a 10 --amount-b 0.05 --confirm`
6. **Check LP balance**: View your LP token holdings for a pair. `pancakeswap-v2-plugin lp-balance --pair <address>`
7. **Remove liquidity**: Burn LP tokens to withdraw your proportional share of the pool. `pancakeswap-v2-plugin remove-liquidity --pair <address> --liquidity <amount> --confirm`
