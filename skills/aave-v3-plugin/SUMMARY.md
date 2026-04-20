**Overview**

Supply assets and borrow against collateral on Aave V3 across Ethereum, Base, Polygon, and Arbitrum — with real-time Health Factor tracking to prevent liquidation.

**Prerequisites**
- onchainos agentic wallet connected
- Some tokens on a supported chain — Ethereum, Base (default), Polygon, or Arbitrum

**How it Works**
1. **Supply**:
   - 1.1 **Check available markets**: Browse assets with supply APY, borrow rate, and utilization. `aave-v3-plugin get-reserves`
   - 1.2 **Supply assets**: Deposit tokens to earn yield — ERC-20 approval fires automatically. `aave-v3-plugin supply --asset USDC --amount <amount> --confirm`
   - 1.3 **Monitor your position**: View total collateral, debt, borrow power, and Health Factor. `aave-v3-plugin positions`
2. **Borrow** (requires collateral supplied first; Health Factor must stay above 1.0):
   - 2.1 **Borrow**: Draw against your supplied collateral — choose variable or stable rate. `aave-v3-plugin borrow --asset WETH --amount <amount> --rate-mode variable --confirm`
   - 2.2 **Repay**: Return borrowed assets and free up collateral — use `--amount max` to repay in full. `aave-v3-plugin repay --asset WETH --amount <amount> --confirm`
   - 2.3 **Withdraw collateral**: Reclaim supplied assets — only possible while Health Factor stays above 1.0. `aave-v3-plugin withdraw --asset USDC --amount <amount> --confirm`
