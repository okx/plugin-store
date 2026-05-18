# FILL-IN — maple-onchainos implementation checklist

Status as of v0.1.0: **all 5 tools implemented; live-read against Ethereum mainnet validated.**

## Tool 1 — `listSupportedPools`

- [x] Returns curated set (syrupUSDC, syrupUSDT on Ethereum).
- [x] Optional `chain` filter.
- [ ] Roadmap: Base pool addresses (need to confirm canonical user-facing addresses; SDK only ships infrastructure addresses).

## Tool 2 — `getPool`

- [x] viem ERC-4626 reads: `symbol`, `asset`, `totalAssets`, `totalSupply`, `previewDeposit(1 unit)`.
- [x] Pool address validated against curated registry → `UNKNOWN_POOL` on miss.
- [x] `formatUnits` applied to `totalAssets` using known decimals.
- [ ] Roadmap: surface APY by reading `WithdrawalManager` or computing from `previewDeposit` over time windows.

## Tool 3 — `getPosition`

- [x] Reads `balanceOf` and `convertToAssets`.
- [x] Returns 0-position note with next-step suggestion when shares==0.
- [ ] Roadmap: surface pending redemption requests (Maple's withdrawal queue) — needs WithdrawalManager ABI.

## Tool 4 — `buildDeposit`

- [x] Reads underlying asset from `pool.asset()` (not user-supplied — prevents spoofed-asset attacks).
- [x] Allowance check → emit approve `pending_sign` (Step 1 of 2) if insufficient.
- [x] Underlying-balance pre-flight (INSUFFICIENT_BALANCE before broadcast).
- [x] Native-balance gate (P1-14, no-op for value=0 but wired uniformly).
- [x] viem `encodeFunctionData` for `deposit(uint256,address)`.
- [x] Receiver defaults to walletAddress; optional override validated.
- [ ] Roadmap: `permit` (ERC-2612) variant to skip the approve tx on permit-enabled tokens (USDC supports it, USDT does not).

## Tool 5 — `buildQueueWithdrawal`

- [x] Validates user holds enough shares before emitting.
- [x] Calls `requestRedeem(shares, owner)` — Maple's cycle-based queue.
- [x] Description makes the "not instant" semantics explicit (P1-16 — fee-mutation/flow ambiguity surfaced in description field).
- [ ] Roadmap: `claimRedemption` tool for after the cycle settles (need WithdrawalManager `processRedemptions` / `claim` ABI).

## Cross-cutting

- [x] All EVM addresses pass through `lc()` to satisfy viem's strict EIP-55.
- [x] `parseUnits` always paired with a known `decimals` from the pool registry (no user-supplied decimals — that class of error is dodged structurally, A7-equivalent).
- [x] Error taxonomy matches SKILL.md catalog (UNSUPPORTED_CHAIN, UNKNOWN_POOL, INSUFFICIENT_BALANCE, NO_POSITION, RPC_ERROR).
- [x] No local-signing residue (only `viem.encodeFunctionData` + `viem.readContract`).

## API quirks checklist (P1-16)

The Maple SDK has no boolean fee-mutation flags (unlike deBridge's `prependOperatingExpenses`). Notes for future tools:

| Pattern | Status |
|---|---|
| Fee-mutation flags | n/a — Maple is fee-free at the protocol entrypoint; APY is implicit in `convertToAssets` |
| Slippage | n/a — pool deposits are mint-at-current-rate, no slippage param |
| Affiliate fee | n/a |
| Pool whitelisting | **YES** — some Maple pools have `PoolPermissionManager` gating (KYC, allowlist). syrupUSDC/syrupUSDT are open; if we add the "lender-only" Maple v2 pools later, we'd need to surface `canDeposit(user)` checks in `buildDeposit` |
| Withdrawal cycle timing | **YES** — `requestRedeem` is queued; `description` makes this explicit |

## Known SDK delta

`maple-labs/maple-js` v3 `generateUnsignedTransactionData` uses **ethers v5** (`@ethersproject/*` packages). This skill uses **viem** instead. The output (an `unsigned_tx` with `to/data/value`) is functionally equivalent. The SDK additionally fills in `nonce`, `gasLimit`, `maxFeePerGas`, `maxPriorityFeePerGas`, `chainId`, `type: 2` — Onchain OS fills these at broadcast time, so we strip them from the envelope.
