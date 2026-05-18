# Swap API Deep-Dive

The Swap API (`/swap/approval`) is the recommended integration path for most developers. It abstracts swap + bridge + swap into a single call and returns executable calldata.

## Base URLs
- Mainnet: `https://app.across.to/api`
- Testnet: `https://testnet.across.to/api`

## Integrator ID (required for production)
Before launch, register and obtain your 2-byte hex integrator ID (e.g., `0xdead`). This ID is used for tracking and support.

## Caching and Liveness
- Do not cache `/swap/approval` responses. Quotes are derived from on-chain state and can change every block.
- `/deposit/status` is the only stateful endpoint (indexer-backed). See `deposit-tracking.md`.

## Trade Types
- `exactInput`: You specify the exact input amount. The API returns expected output after fees.
- `minOutput`: You specify the minimum output amount you will accept. The API computes required input.
- `exactOutput`: You specify the exact output amount required. If the API cannot deliver that exact amount, the transaction is cancelled and funds are refunded.

## Slippage
- `slippage=auto` (default) lets the API choose an optimal value.
- A numeric slippage value must be between `0` and `1` — this is a **ratio, not a percentage**. `0.005` = 0.5%, `0.01` = 1%, `0.5` = 50%. A common mistake is passing `0.5` intending 0.5% — that would allow 50% slippage.
- Numeric slippage is split evenly across origin and destination swaps when both swaps exist.
- If only one swap happens (origin-only or destination-only), the full slippage is applied to that single leg.

## GET /swap/approval
Returns data required to execute a crosschain swap. If the input token requires approval, `approvalTxns` is included.

### Required query parameters
| Parameter | Description |
|-----------|-------------|
| `tradeType` | `exactInput`, `minOutput`, or `exactOutput` |
| `amount` | Required amount in smallest units. For `exactInput`, this is the input amount; otherwise it is the output amount |
| `inputToken` | Input token address on origin chain |
| `outputToken` | Output token address on destination chain |
| `originChainId` | Origin chain ID |
| `destinationChainId` | Destination chain ID |
| `depositor` | Address initiating the swap |

### Optional query parameters
| Parameter | Description |
|-----------|-------------|
| `recipient` | Address receiving output token (defaults to `depositor`) |
| `appFee` | Integrator fee percentage (0 to 1), denominated in output token |
| `appFeeRecipient` | Address receiving app fees (required if `appFee` is set) |
| `integratorId` | 2-byte hex string identifying the integrator (e.g., `0xdead`) |
| `refundAddress` | Address to receive refunds (defaults to `depositor`) |
| `refundOnOrigin` | Boolean override for refund chain. If omitted, defaults depend on route type (B2B/A2B => origin; B2A/A2A => destination). Refund recipient priority: `refundAddress` > `recipient` > `depositor` |
| `slippage` | `auto` (default) or numeric decimal between 0 and 1 |
| `skipOriginTxEstimation` | If true, skip origin swap estimation. Defaults differ between description and schema; set explicitly to avoid ambiguity |
| `strictTradeType` | If true, strictly follow the provided `tradeType` |
| `excludeSources` | Array of swap sources to exclude (see `/swap/sources`) |
| `includeSources` | Array of swap sources to include (see `/swap/sources`) |

### Response (key fields)
- `crossSwapType`: `bridgeableToBridgeable`, `bridgeableToBridgeableIndirect`, `bridgeableToAny`, `anyToBridgeable`, `anyToAny`
- `approvalTxns[]`: ERC-20 approvals to submit before the swap (`chainId`, `to`, `data`)
- `checks`: allowance and balance checks
- `steps`: `originSwap`, `bridge`, `destinationSwap` (present depending on route)
- `fees`: `total`, `totalMax`, and `originGas` breakdowns
- `inputAmount`, `maxInputAmount`, `expectedOutputAmount`, `minOutputAmount`
- `expectedFillTime` (seconds)
- `swapTx`: executable transaction data (`chainId`, `to`, `data`, `gas`, `maxFeePerGas`, `maxPriorityFeePerGas`, `simulationSuccess`)
- `quoteExpiryTimestamp`
- `id` (unique request identifier)

### Errors
- `400`: bad request due to invalid input
- `4XX`: insufficient liquidity for same-asset bridging

## POST /swap/approval (embedded actions)
Use this to build embedded crosschain actions that execute on the destination chain immediately after the swap.

- Query parameters are the same as GET, but the OpenAPI reference describes `amount` as the required output-token amount for POST. If you rely on exact-input semantics, verify behavior with the API before shipping.
- Request body includes an `actions` array.

### Request body
```json
{
  "actions": [
    {
      "target": "0x...",
      "functionSignature": "function transfer(address,uint256)",
      "args": [
        { "value": "0xRecipient", "populateDynamically": false },
        { "value": "0", "populateDynamically": true, "balanceSourceToken": "0xToken" }
      ],
      "value": "0",
      "isNativeTransfer": false
    }
  ]
}
```

### Notes
- The OpenAPI spec for POST uses an upper-case `crossSwapType` enum (`BRIDGEABLE_TO_BRIDGEABLE`, `BRIDGEABLE_TO_ANY`, `ANY_TO_BRIDGEABLE`, `ANY_TO_ANY`).
- Treat the POST response as matching the GET response unless your integration depends on the specific enum casing.

## End-to-End Example: Bridge USDC from Arbitrum to Base

This example shows the complete flow for a crosschain USDC transfer.

### 1. Get a quote

```
GET https://app.across.to/api/swap/approval?tradeType=exactInput&amount=10000000&inputToken=0xaf88d065e77c8cC2239327C5EDb3A432268e5831&outputToken=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&originChainId=42161&destinationChainId=8453&depositor=0xYourAddress&slippage=auto
```

- `amount=10000000` = 10 USDC (6 decimals)
- `inputToken` = USDC on Arbitrum
- `outputToken` = USDC on Base

### 2. Check for approval transactions

If the response includes `approvalTxns`, submit each one and wait for confirmation before proceeding.

```javascript
for (const approvalTx of response.approvalTxns) {
  const tx = await wallet.sendTransaction({
    to: approvalTx.to,
    data: approvalTx.data,
    chainId: approvalTx.chainId,
  });
  await tx.wait();
}
```

### 3. Submit the swap transaction

```javascript
const swapTx = await wallet.sendTransaction({
  to: response.swapTx.to,
  data: response.swapTx.data,
  chainId: response.swapTx.chainId,
  gasLimit: response.swapTx.gas,
});
const receipt = await swapTx.wait();
```

### 4. Track the deposit

```
GET https://app.across.to/api/deposit/status?depositTxnRef={receipt.transactionHash}
```

Poll every 10 seconds until `status` = `"filled"`.

## Related endpoints
```
GET /swap/chains
GET /swap/tokens
GET /swap/sources
GET /swap/sources?chainId={id}
```

Use these to populate UIs and validate inputs before calling `/swap/approval`.
