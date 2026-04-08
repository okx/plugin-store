# sushiswap-v3

SushiSwap V3 plugin for onchainos — swap tokens and manage concentrated liquidity positions across 38+ EVM chains.

## Operations

- `quote` — Get a swap quote (no gas)
- `swap` — Execute token swap
- `get-pools` — Query pool addresses
- `get-positions` — List LP positions
- `add-liquidity` — Add concentrated liquidity
- `remove-liquidity` — Remove liquidity
- `collect-fees` — Collect accumulated fees

## Supported Chains

Base (8453), Ethereum (1), Arbitrum (42161), BSC (56), Polygon (137), Optimism (10), Avalanche (43114)

## Contracts (all chains, same addresses)

- Factory: `0xc35DADB65012eC5796536bD9864eD8773aBc74C4`
- SwapRouter: `0xFB7eF66a7e61224DD6FcD0D7d9C3be5C8B049b9f`
- QuoterV2: `0xb1E835Dc2785b52265711e17fCCb0fd018226a6e`
- NonfungiblePositionManager: `0x80C7DD17B01855a6D2347444a0FCC36136a314de`

## Usage

```bash
sushiswap-v3 quote --token-in WETH --token-out USDC --amount-in 1000000000000000 --chain 8453
sushiswap-v3 swap --token-in WETH --token-out USDC --amount-in 50000000000000 --chain 8453
sushiswap-v3 get-positions --chain 8453
```

See `skills/sushiswap-v3/SKILL.md` for complete documentation.

## Build

```bash
cargo build --release
```

## License

MIT
