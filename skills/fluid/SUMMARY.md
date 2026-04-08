
# fluid
Fluid Protocol DEX + Lending integration for supply/withdraw to ERC-4626 fTokens and swap via Fluid AMM across Base, Ethereum, and Arbitrum.

## Highlights
- ERC-4626 fToken lending with yield earning (fUSDC, fWETH, fGHO, fEURC)
- Concentrated AMM DEX for token swaps (EURC/USDC, wstETH/ETH, weETH/ETH)
- Multi-chain support across Base, Ethereum, and Arbitrum
- Direct on-chain data via resolver contracts with no external APIs
- Safe dry-run simulation before all write operations
- Automatic approval handling for ERC-20 deposits
- Real-time lending positions and market data
- Liquidation-risk protection with vault operations limited to dry-run only

