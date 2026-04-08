# ion-protocol
Ion Protocol CDP lending plugin for LRT/LST collateral on Ethereum Mainnet with 4 active pools for borrowing against liquid restaking tokens.

## Highlights
- Deposit LRT collateral (rsETH, rswETH, ezETH, weETH) to borrow wstETH or WETH
- Supply wstETH or WETH to earn lending yield (up to ~32% APY on rsETH pool)
- CDP-style lending system with isolated collateral pools, not Aave-style pooling
- 4 active pools: rsETH/wstETH, rswETH/wstETH, ezETH/WETH, weETH/wstETH
- Full position management: lend, borrow, repay, withdraw collateral
- Real-time pool rates and TVL monitoring via on-chain data
- Ethereum Mainnet only with direct RPC integration
- Complete transaction flow from collateral approval to debt repayment

