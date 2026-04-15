# curve-plugin
A Curve DEX plugin for swapping stablecoins, managing liquidity, and querying pool data across Ethereum, Arbitrum, Base, Polygon, and BSC.

## Highlights
- Swap stablecoins on Curve Finance across 5 major chains
- Add and remove liquidity from Curve pools with automatic token approvals
- Query pool APY, TVL, and detailed information from official Curve API
- Get LP token balances across all Curve positions with Multicall3 batching
- Human-readable amounts with automatic decimal resolution from pool data
- Safety-first design with confirmation gates for all write operations
- Built-in slippage protection and price impact warnings
- Support for both proportional and single-coin liquidity withdrawal

