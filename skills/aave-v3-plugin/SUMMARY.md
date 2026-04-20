**Overview**

Supply assets and borrow against collateral on Aave V3 ($43B+ TVL) across Ethereum, Base, Polygon, and Arbitrum — with real-time Health Factor tracking to prevent liquidation.

**Prerequisites**
- onchainos agentic wallet connected
- EVM wallet with funds on Ethereum (1), Base (8453, default), Polygon (137), or Arbitrum (42161)
- For borrowing: sufficient collateral supplied first; Health Factor must stay above 1.0

**How it Works**

Supplying and earning yield:
1. **Check available markets**: Browse assets with supply APY, borrow rate, and utilization. `aave-v3-plugin get-reserves`
2. **Supply assets**: Deposit tokens to earn yield — ERC-20 approval fires automatically. `aave-v3-plugin supply --asset USDC --amount 100 --confirm`
3. **Monitor your position**: View total collateral, debt, borrow power, and Health Factor. `aave-v3-plugin positions`

Borrowing:
4. **Borrow**: Draw against your supplied collateral — choose variable or stable rate. `aave-v3-plugin borrow --asset WETH --amount 0.05 --rate-mode variable --confirm`
5. **Repay**: Return borrowed assets and free up collateral — use `--amount max` to repay in full. `aave-v3-plugin repay --asset WETH --amount 0.05 --confirm`
6. **Withdraw collateral**: Reclaim supplied assets — only possible while Health Factor stays above 1.0. `aave-v3-plugin withdraw --asset USDC --amount 100 --confirm`
