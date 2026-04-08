# navi-protocol

NAVI Protocol CLI plugin — lend and borrow on Sui's leading DeFi lending protocol.

## Overview

NAVI Protocol is the first native one-stop liquidity protocol on Sui blockchain, offering:
- Variable-rate lending and borrowing
- Health factor-based liquidation model
- E-Mode for correlated asset pairs
- 30+ supported assets

## Installation

```bash
# Build from source
cd skills/navi-protocol
cargo build --release
./target/release/navi-protocol --help
```

## Usage

### View all lending markets
```bash
navi-protocol reserves
```

Sample output:
```
Asset      Price    Supply%  Borrow%   Util%  TotalSupply  TotalBorrow  MaxLTV%
------------------------------------------------------------------------------------------
SUI       $3.5210     4.21%    6.50%  62.30%      45.20M       28.17M      65.0%
nUSDC     $1.0001     5.89%    8.20%  72.10%      12.50M        9.02M      80.0%
...
```

### Check a wallet's positions
```bash
navi-protocol positions --wallet 0x<sui-address>
```

### Preview a supply transaction
```bash
navi-protocol supply --asset SUI --amount 10 --wallet 0x<address>
```

### Preview a borrow transaction
```bash
navi-protocol borrow --asset nUSDC --amount 100 --wallet 0x<address>
```

## Write Operations Note

Sui is not currently supported by onchainos CLI for transaction execution.
All write commands (supply, withdraw, borrow, repay) output the corresponding
Move call preview. To execute transactions:
- Use the NAVI web app: https://app.naviprotocol.io
- Use the NAVI TypeScript SDK: `npm install @naviprotocol/lending`

## Key Addresses (Sui Mainnet)

| Contract | Address |
|----------|---------|
| Protocol Package (latest) | fetched from `open-api.naviprotocol.io/api/package` |
| Storage | `0xbb4e2f4b6205c2e2a2db47aeb4f830796ec7c005f88537ee775986639bc442fe` |
| Price Oracle | `0x1568865ed9a0b5ec414220e8f79b3d04c77acc82358f6e5ae4635687392ffbef` |

## Chain

Sui Mainnet — Chain ID 784

## License

MIT
