# exactly-protocol
Fixed-rate and floating-rate lending protocol enabling deposits at fixed APY locked until maturity, plus variable rate pools on Optimism and Ethereum.

## Highlights
- Fixed-rate lending with maturity-based pools for predictable returns
- Floating-rate pools using ERC-4626 standard for flexible deposits
- Deployed on Optimism (primary, lower gas) and Ethereum Mainnet
- Supports WETH, USDC, OP, wstETH, and WBTC markets
- Explicit collateral enabling required before borrowing (unlike Aave)
- Weekly maturity timestamps aligned to Thursday UTC boundaries
- Early withdrawal penalties for fixed deposits before maturity
- Zero-slippage fixed-rate borrowing with penalty fees after maturity

