# Segment Finance Plugin

Segment Finance lending and borrowing on BNB Chain (BSC). This is a Compound V2 fork with seToken markets.

## Supported Operations

- `get-markets` — List all markets with supply/borrow APY and utilization
- `get-positions` — View your current supply and borrow positions
- `supply` — Supply assets to earn interest
- `withdraw` — Redeem supplied assets
- `borrow` — Borrow against collateral
- `repay` — Repay borrowed assets
- `enter-market` — Enable asset as collateral

## Supported Chain

- BNB Chain (BSC), chain ID 56

## Supported Assets

BNB, USDT, USDC, BTC (BTCB), ETH

## Usage

```bash
segment-finance get-markets --chain 56
segment-finance get-positions --chain 56
segment-finance supply --asset USDT --amount 10.0 --chain 56 --dry-run
segment-finance withdraw --asset USDT --amount 5.0 --chain 56 --dry-run
segment-finance borrow --asset USDT --amount 5.0 --chain 56 --dry-run
segment-finance repay --asset USDT --amount 5.0 --chain 56 --dry-run
segment-finance enter-market --asset USDT --chain 56 --dry-run
```

## License

MIT
