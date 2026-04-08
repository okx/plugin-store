# Moonwell Flagship Plugin

Moonwell is an open, non-custodial lending and borrowing protocol (Compound V2 fork) built on Base, Optimism, Moonbeam, and Moonriver. This plugin enables supply, redeem, borrow (dry-run), repay (dry-run), and WELL reward claiming via onchainos.

## Supported Chains

- Base (8453) — primary test chain
- Optimism (10)
- Moonbeam (1284)

## Supported Assets (Base)

| Symbol | mToken |
|--------|--------|
| USDC | `0xEdc817A28E8B93B03976FBd4a3dDBc9f7D176c22` |
| WETH | `0x628ff693426583D9a7FB391E54366292F509D457` |
| cbETH | `0x3bf93770f2d4a794c3d9EBEfBAeBAE2a8f09A5E5` |
| USDbC | `0x703843C3379b52F9FF486c9f5892218d2a065cC8` |
| DAI | `0x73b06D8d18De422E269645eaCe15400DE7462417` |

## Commands

```bash
moonwell markets --chain 8453
moonwell positions --chain 8453
moonwell supply --asset USDC --amount 0.01 --chain 8453
moonwell redeem --asset USDC --mtoken-amount 100.0 --chain 8453
moonwell borrow --asset USDC --amount 5.0 --chain 8453 --dry-run
moonwell repay --asset USDC --amount 5.0 --chain 8453 --dry-run
moonwell claim-rewards --chain 8453
```

## Build

```bash
cargo build --release
```

## License

MIT
