# NAVI Protocol Plugin — Design Document

## Protocol Overview
NAVI Protocol is the leading one-stop liquidity protocol on Sui blockchain, offering:
- Lending/borrowing with dynamic interest rates
- Health factor-based liquidation model (similar to Aave)
- E-Mode for correlated asset pairs at higher LTV
- Isolated pools for long-tail assets
- Flash loans

## Key Contract Addresses (Sui Mainnet)

### Protocol Package
- Latest: fetched dynamically from `https://open-api.naviprotocol.io/api/package`
- Default (v21): `0xee0041239b89564ce870a7dec5ddc5d114367ab94a1137e90aa0633cb76518e0`
- Latest (v22): `0x1e4a13a0494d5facdbe8473e74127b838c2d446ecec0ce262e2eddafa77259cb`

### Core Object IDs
- **StorageId**: `0xbb4e2f4b6205c2e2a2db47aeb4f830796ec7c005f88537ee775986639bc442fe`
- **PriceOracle**: `0x1568865ed9a0b5ec414220e8f79b3d04c77acc82358f6e5ae4635687392ffbef`
- **IncentiveV2**: `0xf87a8acb8b81d14307894d12595541a73f19933f88e1326d5be349c7a6f7559c`
- **IncentiveV3**: `0x62982dad27fb10bb314b3384d5de8d2ac2d72ab2dbeae5d801dbdb9efa816c80`
- **ReserveParentId**: `0xe6d4c6610b86ce7735ea754596d71d72d10c7980b5052fc3c8cdf8d09fea9b4b`
- **UIGetter**: `0xf56370478288b5e1838769929823efaed88bf7ad89040d8a2ac391d6bd0aa2f2`

### Pool Object IDs (from SDK address.ts)
| Asset | assetId | poolId |
|-------|---------|--------|
| SUI   | 0 | 0x96df0fce3c471489f4debaaa762cf960b3d97820bd1f3f025ff8190730e958c5 |
| wUSDC | 1 | 0xa02a98f9c88db51c6f5efaaf2261c81f34dd56d86073387e0ef1805ca22e39c8 |
| USDT  | 2 | 0x0e060c3b5b8de00fb50511b7a45188c8e34b6995c01f69d98ea5a466fe10d103 |
| WETH  | 3 | 0x71b9f6e822c48ce827bceadce82201d6a7559f7b0350ed1daa1dc2ba3ac41b56 |
| NAVX  | 7 | 0xc0e02e7a245e855dd365422faf76f87d9f5b2148a26d48dda6e8253c3fe9fa60 |
| nUSDC | 10 | 0xa3582097b4c57630046c0c49a88bfc6b202a3ec0a9db5597c31765f7563755a8 |

## REST API Endpoints (NAVI Open API)
- **Pools data**: `https://open-api.naviprotocol.io/api/navi/pools`
  - Returns: supply rate, borrow rate, total supply, total borrow, utilization, APY, oracle price
  - Rate scale: 1e27 (divide by 1e25 to get percentage)
- **Latest package**: `https://open-api.naviprotocol.io/api/package`
  - Returns: `{ packageId: "0x..." }`

## Sui JSON-RPC Helpers
- Endpoint: `https://fullnode.mainnet.sui.io`
- Method `sui_getObject`: fetch pool or reserve objects
- Method `suix_getDynamicFieldObject`: fetch user borrow/supply balances
  - Supply balance parent: `reserve.supplyBalanceParentId` (per-asset)
  - Borrow balance parent: `reserve.borrowBalanceParentId` (per-asset)
- Method `sui_devInspectTransactionBlock`: read health factor via move view call
  - Target: `{uiGetter}::logic_getter_unchecked::user_health_factor`

## Move Call Structure (Supply)
```
package: <latest_protocol_package>
module: lending_core  (or pool)
function: supply
type_args: [<CoinType>]
args:
  - storage object id
  - pool object id (for asset)
  - oracle object id
  - incentive_v2 object id
  - clock (0x6)
  - coin_object (user's coin)
  - amount (u64)
```

## Interest Rate Calculation
- Raw rates are stored as Ray (1e27 scaling)
- Supply APY = currentSupplyRate / 1e25 (percentage)
- Borrow APY = currentBorrowRate / 1e25 (percentage)
- Utilization = totalBorrow * currentBorrowIndex / (totalSupply * currentSupplyIndex)

## Health Factor
- Computed on-chain via devInspect move call
- > 1.0 = safe; <= 1.0 = liquidatable
- Equivalent to: sum(collateral * threshold * price) / sum(debt * price)

## Implementation Notes
- Sui is NOT supported by onchainos CLI — all write operations are preview-only
- Pool data is best fetched from REST API (has APY, price, utilization pre-computed)
- User positions require Sui RPC dynamic field lookups per asset per user
- Health factor requires devInspect RPC call (read-only, no gas needed)
