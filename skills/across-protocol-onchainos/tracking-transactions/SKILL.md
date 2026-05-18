# Deposit Tracking

Track the lifecycle of any Across deposit using the `/deposit/status` endpoint.

## Endpoint

```
GET https://app.across.to/api/deposit/status
```

## Query Parameters (use one of these combinations)

Option A: by transaction hash
| Parameter | Description |
|-----------|-------------|
| `depositTxnRef` | The deposit transaction hash emitted by `FundsDeposited` |

Option B: by chain + deposit ID
| Parameter | Description |
|-----------|-------------|
| `originChainId` | Chain ID where the deposit originated |
| `depositId` | Deposit ID emitted by `FundsDeposited` |

Note: The API description states `depositTxnRef` is not required when using `originChainId` + `depositId`, even though the OpenAPI parameter list marks it as required. Set one of the two options explicitly.

## Status Values

| Status | Meaning |
|--------|---------|
| `pending` | Deposit has not been filled yet |
| `filled` | Filled on destination chain; recipient has received funds |
| `expired` | Deposit expired and will not be filled; refund will be processed on origin chain |
| `refunded` | Depositor has been refunded on origin chain |

## Response Fields (key)
- `status`
- `fillTxnRef` (only present when `status` is `filled`)
- `destinationChainId`
- `originChainId`
- `depositId`
- `depositTxnRef`
- `depositRefundTxnRef`
- `actionsSucceeded`
- `pagination` (currentIndex, maxIndex for batched results)

## Latency
This endpoint is backed by an indexing service that polls events on a 10-second cadence. Expect 1 to 15 seconds of latency after a deposit transaction is confirmed before the status updates.

## Recommended Integration Pattern
1. Store `depositTxnRef` (or `originChainId` + `depositId`) for each transaction your app initiates.
2. Poll `GET /deposit/status` **every 10 seconds** to update your UI. Do not poll more frequently — the indexer cadence is 10 seconds.
3. On mainnet, expect fills in **~2 seconds** — most deposits resolve on the first or second poll.
4. Show the fill transaction hash (`fillTxnRef`) when status is `filled`.
5. Handle `expired` by informing the user that a refund is in progress.
6. If using embedded actions (`POST /swap/approval`), check the `actionsSucceeded` field to verify destination actions executed.

## Bulk Retrieval
To get all deposits for a given wallet:

```
GET https://app.across.to/api/deposits?depositor={address}
```

Supports:
- `limit`: maximum number of deposits to return
- `skip`: number of deposits to skip
- `depositor`: wallet address to filter by

Key fields returned per deposit include:
- `depositId`, `originChainId`, `destinationChainId`
- `depositor`, `recipient`
- `inputToken`, `inputAmount`
- `outputToken`, `outputAmount`
- `depositTxHash`, `depositBlockNumber`, `depositBlockTimestamp`
- `status`, `fillTx`, `fillBlockTimestamp`
- `depositRefundTxHash`
- `quoteTimestamp`, `fillDeadline`, `exclusivityDeadline`, `exclusiveRelayer`
- `swap` and `bridge` fee fields (USD) and gas fee fields

## Important Notes
- For best results, use the `deposit/status` endpoint on mainnet only; testnet indexing may be incomplete.
- This endpoint is stateful and indexer-backed, unlike most other Across APIs.
