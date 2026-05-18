# Suggested Fees API (Legacy / Advanced)

Use the `/suggested-fees` API when you control your own swap infrastructure and only need Across for the bridge leg. This is the lower-level path: you construct `depositV3` calls manually.

For most integrations, use the Swap API (`/swap/approval`).

## When to use this API
- You have your own DEX aggregator and only need bridge fee quotes.
- You want bridgeable-to-bridgeable transfers (same asset across chains).
- You need fine-grained control over `depositV3` call construction.

## GET /suggested-fees
Returns suggested fees based on `inputToken` + `outputToken`, `originChainId`, `destinationChainId`, and `amount`. Also includes data used to compute the fees.

### Required parameters
| Parameter | Description |
|-----------|-------------|
| `inputToken` | Token address on origin chain (use wrapped token for native assets, e.g., WETH) |
| `outputToken` | Token address on destination chain (use wrapped token for native assets, e.g., WETH) |
| `originChainId` | Origin chain ID |
| `destinationChainId` | Destination chain ID |
| `amount` | Amount in token native decimals (USDC 1e6, WETH 1e18) |

### Optional parameters
| Parameter | Description |
|-----------|-------------|
| `recipient` | Recipient of the deposit (EOA or contract). If this is an EOA and `message` is set, the API returns an error |
| `message` | Calldata for contract recipients; passed to `handleAcrossMessage()` on the recipient |
| `relayer` | Override the relayer used to simulate `fillRelay()` for gas estimation |
| `timestamp` | Quote timestamp used to compute LP fees. Must be close to current time and <= current mainnet block timestamp. Defaults to latest mainnet block timestamp |

### Key response fields
- `totalRelayFee` (pct + total)
- `relayerCapitalFee` (pct + total)
- `relayerGasFee` (pct + total)
- `lpFee` (pct + total)
- `timestamp` (quote timestamp for LP fee calculation)
- `isAmountTooLow`
- `quoteBlock`
- `spokePoolAddress`
- `exclusiveRelayer`
- `exclusivityDeadline`
- `expectedFillTimeSec`
- `fillDeadline`
- `limits` (see `TransferLimits` below)

## GET /limits
Returns transfer limits for `inputToken`, `outputToken`, `originChainId`, and `destinationChainId`.

### TransferLimits fields
- `minDeposit`
- `maxDeposit`
- `maxDepositInstant`
- `maxDepositShortDelay`
- `recommendedDepositInstant`

## GET /available-routes
Returns available bridgeable routes. Filter by any combination of:
- `originChainId`
- `destinationChainId`
- `originToken`
- `destinationToken`

Each route includes token addresses and symbols for both chains.

## Manual depositV3 flow
1. Call `/suggested-fees` to get fee quote and SpokePool address.
2. Approve the input token for the origin SpokePool (`spokePoolAddress` from the response) if needed.
3. Construct and submit `depositV3` on the origin SpokePool with the quote parameters.
4. Track status via `GET /deposit/status`.

### depositV3 function signature

```solidity
function depositV3(
    address depositor,           // Address initiating the deposit
    address recipient,           // Address receiving tokens on destination chain
    address inputToken,          // Token address on origin chain
    address outputToken,         // Token address on destination chain
    uint256 inputAmount,         // Amount of inputToken to deposit
    uint256 outputAmount,        // Minimum output amount (inputAmount - totalRelayFee.total)
    uint256 destinationChainId,  // Destination chain ID
    address exclusiveRelayer,    // From /suggested-fees response (address(0) if none)
    uint32 quoteTimestamp,       // timestamp from /suggested-fees response
    uint32 fillDeadline,         // fillDeadline from /suggested-fees response
    uint32 exclusivityDeadline,  // exclusivityDeadline from /suggested-fees response
    bytes message                // Calldata for destination contract (empty bytes if none)
)
```

### Mapping /suggested-fees response to depositV3 parameters

| depositV3 param | Source |
|-----------------|--------|
| `inputAmount` | Your original `amount` |
| `outputAmount` | `amount - totalRelayFee.total` |
| `exclusiveRelayer` | `exclusiveRelayer` from response |
| `quoteTimestamp` | `timestamp` from response |
| `fillDeadline` | `fillDeadline` from response |
| `exclusivityDeadline` | `exclusivityDeadline` from response |
| `message` | `0x` (empty) unless using crosschain messages |

Call `depositV3` on the SpokePool contract at `spokePoolAddress` from the `/suggested-fees` response.

## Important Notes
- Do not cache `/suggested-fees` responses.
- If `recipient` is an EOA, do not set `message`.
- For embedded crosschain actions with this API, use the legacy guide referenced in the Across docs.
