# X Layer DeFi Protocol Map

Reference document for known DeFi protocols on X Layer (chain ID 196).

## Chain Info

| Property | Value |
|----------|-------|
| Chain ID | 196 |
| Native Token | OKB |
| Consensus | zkEVM (Polygon CDK) |
| Gas Cost | Near-zero (~$0.001 per tx) |
| Finality | ~10 seconds |
| Bridge | OKX Bridge (native), third-party bridges |

## Known DEX Protocols

### Uniswap V3 (X Layer deployment)
- Type: AMM / Concentrated Liquidity
- Pairs: OKB/USDC, OKB/USDT, USDC/USDT, OKB/WETH
- Router: Check onchainos for current addresses

### Native DEXes
- Several native DEXes may be deployed on X Layer
- Use `onchainos token search --chain 196` to discover current pools

## Stablecoin Addresses on X Layer

Use `onchainos token search --query <SYMBOL> --chain 196 --format json` to get current contract addresses.

Common stablecoins:
- USDC (bridged)
- USDT (bridged)
- DAI (bridged)

## Bridge Options

### OKX Native Bridge
- Source chains: Ethereum, BSC, Polygon, Arbitrum, Avalanche
- Tokens: USDC, USDT, OKB, WETH
- Speed: ~2-5 minutes
- Fee: Very low (subsidized by OKX)

### Third-Party Bridges
- Available via onchainos cross-chain swap aggregator
- Aggregates multiple bridge protocols for best rates

## Yield Opportunities

### LP Farming
- Provide liquidity to DEX pairs
- Earn trading fees + potential liquidity mining rewards

### Lending (if available)
- Deposit stablecoins to earn interest
- Check `onchainos token search --chain 196` for lending protocol tokens

## Risk Factors

1. **Smart Contract Risk**: New protocols on X Layer may not be audited
2. **Bridge Risk**: Cross-chain bridges are historically attack vectors
3. **Liquidity Risk**: Newer pools may have thin liquidity
4. **Depeg Risk**: Bridged stablecoins depend on bridge security
5. **Impermanent Loss**: LP positions in volatile pairs

## Notes

- X Layer gas is near-zero, making frequent rebalancing economically viable
- OKB is the native gas token — user needs a small amount for gas
- All addresses should be verified via onchainos before use
