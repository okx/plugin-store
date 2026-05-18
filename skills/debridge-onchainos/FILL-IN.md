# FILL-IN — implementation checklist

Status as of v0.1.0: **all 5 tools implemented against the live DLN REST API.**
This file is preserved as a per-tool review checklist for future maintenance,
not a "TODO" list.

## Tool 1 — `listSupportedChains`

- [x] Calls `GET /v1.0/supported-chains-info`.
- [x] Returns `{status:'ok', data: chains[]}`.
- [x] Notes Solana chainId is non-EIP155 (`7565164`).
- [ ] Future: cache for 5 min (chains rarely change).

## Tool 2 — `getQuote`

- [x] Calls `GET /v1.0/dln/order/quote`.
- [x] Validates srcChainId, dstChainId, both token addresses, amount.
- [x] Converts UI amount → wei via `parseUnits(amount, decimals)`.
- [x] Lowercases all addresses at the boundary (`lc()`).
- [x] Surfaces upstream error in `suggestion` for unsupported routes.
- [ ] Future: post-process to extract recommendedAmount/usdValue into a flat summary.

## Tool 3 — `getOrderStatus`

- [x] Calls `GET /v1.0/dln/order/{orderId}`.
- [x] Distinguishes `ORDER_NOT_FOUND` (DLN errorId `UNKNOWN_ORDER`) from other errors.
- [x] Validates orderId as 0x-prefixed 64-char hex.
- [ ] Future: accept origin tx hash too (would require a tx → orderId lookup endpoint).

## Tool 4 — `buildBridge`

- [x] Calls `GET /v1.0/dln/order/create-tx`.
- [x] Uses `walletAddress` for srcChainOrderAuthorityAddress AND dstChainOrderAuthorityAddress
      (single-signer assumption is correct for OnchainOS — both authorities are the same TEE-signed wallet).
- [x] Reads ERC-20 allowance via viem `readContract`.
- [x] Stateful single-step emission: emits approve first if needed, then re-invoke for the order.
- [x] Skips approval entirely for native-asset source (`0x0000…0000`).
- [x] Uses DLN's `tx.to` (the router) as the approve spender.
- [x] Routes signing via `onchainos wallet contract-call`.
- [x] Surfaces orderId in description so the agent can later call `getOrderStatus`.
- [ ] Future: surface `estimation.dstChainTokenOut.recommendedAmount` in description.
- [ ] Future: expose `prependOperatingExpenses=false` mode for fee-payer flows.

## Tool 5 — `buildSameChainSwap`

- [x] Calls `GET /v1.0/chain/transaction`.
- [x] Same allowance-check pattern as buildBridge.
- [x] Defaults slippage = 1% (matches DLN aggregator default).
- [x] Defaults affiliateFeePercent = 0.
- [ ] Future: forward `comparedAggregators` to the agent so it can show price benchmarks.

## Cross-cutting

- [x] All EVM addresses pass through `lc()` to satisfy viem's strict EIP-55.
- [x] Numeric conversions use `parseUnits/formatUnits/toBigInt` boundary helpers.
- [x] DLN base URL overridable via `DEBRIDGE_API_BASE_URL` (useful for staging).
- [x] Error taxonomy matches SKILL.md Error Code Catalog.
- [ ] Future: add `bigintReplacer` test coverage in QA-PIPELINE.

## Known DLN quirks

1. **Compliance gating** — DLN's solver network rejects orders to/from addresses
   on its high-risk allowlist with `errorCode 123 / COMPLIANCE_ADDRESS_BLOCKED`.
   Test addresses like `0x000…0dEaD` trigger this. Always smoke-test with a
   live-ish address (e.g. the user's own wallet) when validating create-tx.

2. **Order indexing lag** — `getOrderStatus` returns `UNKNOWN_ORDER` until the
   origin tx is mined AND DLN's indexer picks it up (typically 10–30 s after
   mining). The status tool handles this case explicitly.

3. **Same-chain swap uses a different router contract per chain** — always
   trust `resp.tx.to` from the API. Do not hardcode the BSC router
   (`0x663DC15D…`) for other chains.
