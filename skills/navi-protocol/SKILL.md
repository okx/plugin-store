---
name: navi-protocol
description: "Supply, borrow, repay on NAVI Protocol (Sui). Commands: reserves, positions, supply, withdraw, borrow, repay. Triggers: navi lend, navi borrow, navi protocol, supply on navi, navi health factor"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# NAVI Protocol

NAVI Protocol is the leading one-stop liquidity protocol on Sui blockchain. It enables users to lend, borrow, and earn yield on crypto assets with transparent on-chain interest rates.

## Commands

### `reserves` — List all lending markets
Shows all active NAVI lending pools with supply APY, borrow APY, utilization rate, and total liquidity.

```bash
navi-protocol reserves
navi-protocol reserves --asset SUI
navi-protocol reserves --json
```

### `positions` — Check user positions and health factor
Shows a wallet's supplied and borrowed balances, plus estimated health factor.

```bash
navi-protocol positions --wallet 0x<sui-address>
navi-protocol positions --wallet 0x<sui-address> --json
```

### `supply` — Preview supplying an asset (Sui write — preview only)
```bash
navi-protocol supply --asset SUI --amount 10
navi-protocol supply --asset nUSDC --amount 100 --wallet 0x<address>
```

### `withdraw` — Preview withdrawing a supplied asset (preview only)
```bash
navi-protocol withdraw --asset SUI --amount 5
navi-protocol withdraw --asset nUSDC --amount max
```

### `borrow` — Preview borrowing an asset (preview only)
```bash
navi-protocol borrow --asset USDT --amount 50
navi-protocol borrow --asset WETH --amount 0.01
```

### `repay` — Preview repaying a borrow (preview only)
```bash
navi-protocol repay --asset USDT --amount 50
navi-protocol repay --asset USDT --amount max
```

## Supported Assets
SUI, wUSDC, USDT, WETH, CETUS, NAVX, nUSDC, ETH, suiUSDT (and more via the reserves command)

## Notes
- Read commands (`reserves`, `positions`) query live Sui mainnet data.
- Write commands output a Move call preview. Sui transaction execution requires the NAVI app (https://app.naviprotocol.io) or the NAVI TypeScript SDK.
- Health factor > 1.0 means safe; <= 1.0 means liquidatable.
