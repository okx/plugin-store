# FILL-IN — camelot-v3-onchainos implementation checklist

Status as of v0.1.0: **all 4 tools implemented; live-read against Arbitrum mainnet validated.**

## Tool 1 — `listSupportedChains`

- [x] Returns Arbitrum entry with SwapRouter + Quoter addresses.
- [ ] Roadmap: add other Algebra-based deployments if Camelot expands.

## Tool 2 — `getTokenInfo`

- [x] viem ERC-20 reads: `symbol`, `decimals`, optional `balanceOf`.
- [x] Optional `walletAddress` for balance lookup.

## Tool 3 — `getQuote`

- [x] viem `simulateContract` against Algebra Quoter (handles state-mutating function via eth_call).
- [x] Returns both `amountOut` and `dynamicFeeBps` (Algebra-specific — fees vary per pool).
- [x] `validateDecimals` enforced (A7).
- [ ] Roadmap: multi-hop quoter (Algebra encodes path as `bytes` with token addresses joined).

## Tool 4 — `buildSwap`

- [x] Allowance check → emit approve `pending_sign` (Step 1 of 2) if insufficient.
- [x] tokenIn balance pre-flight (INSUFFICIENT_BALANCE before broadcast).
- [x] Fresh quote fetched to compute `amountOutMinimum` from `slippageBps` (default 50 bps = 0.50%).
- [x] `deadline` = now + `deadlineSeconds` (default 1200s).
- [x] `limitSqrtPrice = 0` — relies on amountOutMinimum for slippage protection (recommended Algebra pattern).
- [x] `validateDecimals` enforced (A7).
- [x] `nativeBalanceCheck` wired (A8; value=0 here but helper imported uniformly).
- [ ] Roadmap: native-input swaps (WETH unwrap or msg.value path with `multicall(refundETH)`).
- [ ] Roadmap: exact-output swaps (`exactOutputSingle`).
- [ ] Roadmap: multi-hop swaps (`exactInput` with `bytes path`).

## Cross-cutting

- [x] All EVM addresses pass through `lc()` to satisfy viem's strict EIP-55.
- [x] No local-signing residue (only `viem.encodeFunctionData` + viem reads).
- [x] PUBLIC_RPCS pinned to `arbitrum-one.publicnode.com` (P2-15).

## API quirks checklist (P1-16)

Algebra is a Uniswap V3 derivative with one critical difference: **dynamic fees**. Notes:

| Pattern | Status |
|---|---|
| Fixed `fee` field in pool calls | **n/a** — Algebra resolves the fee internally per pool/per swap. `exactInputSingle` struct has NO `fee` field (Uniswap V3 has one). Quoter returns the applied fee in basis points. |
| `slippageBps` default | 50 bps (0.50%) — typical CEX slippage default. Aggressive enough to filter MEV without rejecting normal price moves. |
| `limitSqrtPrice = 0` | Means "no price limit"; rely entirely on `amountOutMinimum`. Setting a real value requires knowing the pool's current sqrt-price; out of scope for v0.1.0. |
| `deadline` | Default 1200s (20 min). Long enough to survive a stuck mempool, short enough to bail on stale state. |
| Pool existence | `getQuote` reverts (`QUOTE_FAILED`) if no Camelot V3 pool exists for the pair. No good way to enumerate pairs without indexer; documented in error suggestion. |

## Algebra vs Uniswap V3

| | Uniswap V3 | Algebra (Camelot V3) |
|---|---|---|
| Fee | Fixed per pool (100/500/3000/10000) | Dynamic per pool per swap |
| `exactInputSingle.fee` | Required (uint24) | Absent (struct has 7 fields not 8) |
| Quoter return | `(amountOut)` | `(amountOut, fee)` |
| SwapRouter | `0xE592427A0AEce92De3Edee1F18E0157C05861564` (multi-chain) | `0x1F721E2E82F6676FCE4eA07A5958cF098D339e18` (Arbitrum) |

If you fork this skill for a Uniswap-V3-style DEX, the `exactInputSingle` ABI must be amended to include `fee` and the Quoter must be swapped.
