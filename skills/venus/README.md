# Venus Core Pool Plugin

Venus Protocol Core Pool integration for onchainos Plugin Store.

Venus is a Compound V2 fork deployed on BNB Smart Chain (BSC). It enables lending and borrowing of various assets.

## Supported Operations

- `get-markets` - List all markets with supply/borrow APY
- `get-positions` - View your supply and borrow positions
- `supply` - Supply assets to earn interest (BNB, USDT, BTC, ETH, USDC)
- `withdraw` - Withdraw supplied assets
- `borrow` - Borrow against collateral
- `repay` - Repay borrowed assets
- `enter-market` - Enable asset as collateral
- `claim-rewards` - Claim XVS token rewards

## Chain

BSC (chain ID 56)

## Usage

```bash
venus get-markets --chain 56
venus supply --asset USDT --amount 10 --chain 56 --dry-run
venus get-positions --chain 56
```
