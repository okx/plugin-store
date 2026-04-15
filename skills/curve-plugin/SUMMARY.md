# curve-plugin
A Curve Finance DEX plugin for swapping stablecoins, managing liquidity positions, and querying pool data across Ethereum, Arbitrum, Base, Polygon, and BSC.

## Highlights
- Swap stablecoins on Curve Finance across 5 major chains
- Add and remove liquidity from Curve pools with automatic token approval handling
- Query pool information, APY rates, and TVL data from official Curve API
- Check LP token balances across all Curve pools with Multicall3 batching
- Get real-time swap quotes with slippage protection
- Human-readable amount inputs (e.g., "1000.0 USDC" instead of raw wei)
- Automatic pool selection by TVL for optimal liquidity
- Dry-run mode for previewing transactions before execution

