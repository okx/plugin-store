# Test Results Report — Raydium AMM Plugin

- Date: 2026-04-05
- DApp supported chains: Solana only
- Solana test chain: mainnet (501)
- Compile: PASS
- Lint: PASS (1 warning W100 — base64 reference, not an error)
- Overall pass standard: Solana DApp — Solana operations must all pass

## Summary

| Total | L1 Compile | L2 Read | L3 Simulate | L4 On-chain | Failed | Blocked |
|-------|-----------|---------|-------------|-------------|--------|---------|
| 8     | 2         | 3       | 1           | 2           | 0      | 0       |

## Detailed Results

| # | Scenario (User Perspective) | Level | Command | Result | TxHash / Notes | Remarks |
|---|----------------------------|-------|---------|--------|----------------|---------|
| 1 | Build plugin for release | L1 | `cargo build --release` | PASS | — | All crates compiled, no errors |
| 2 | Lint plugin code and YAML | L1 | `cargo clean && plugin-store lint .` | PASS | W100: base64 reference warning | 0 errors, 1 warning only |
| 3 | Get USDC price from Raydium | L2 | `raydium get-token-price --mints EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | PASS | `success: true`, USDC price = $1.00 | Price field correct |
| 4 | Get SOL→USDC swap quote for 1 SOL | L2 | `raydium get-swap-quote --input-mint So11... --output-mint EPjFWdd5... --amount 1000000000 --slippage-bps 50` | PASS | `success: true`, outputAmount = 79770199 (~79.77 USDC), routePlan 2 hops | Valid multi-hop route returned |
| 5 | List top 5 pools by liquidity | L2 | `raydium get-pool-list --pool-type all --sort-field liquidity --sort-type desc --page-size 5 --page 1` | PASS | `success: true`, 5 pools returned, `hasNextPage: true` | TVLs range from $15.9M to $44.4M |
| 6 | Dry-run SOL→USDC swap (no broadcast) | L3 | `raydium swap --dry-run --input-mint So11... --output-mint EPjFWdd5... --amount 1000000` | PASS | `ok: true`, `dry_run: true`, `note: "dry_run: tx not built or broadcast"` | No onchainos call made, wallet not resolved |
| 7 | Balance pre-check (guardrails) | L4-pre | `onchainos wallet balance --chain 501` | PASS | Balance > 0.003 SOL threshold | Guardrails satisfied |
| 8 | On-chain SOL→USDC swap (0.001 SOL) | L4 | `raydium swap --input-mint So11... --output-mint EPjFWdd5... --amount 1000000` | PASS | `4RiWKL16piCsq9SmitH81dZju4XcbT9qu6gH7NLPQQeyMXfC8a4yGuZoskpcXQgai2jJVT9LNg7DVwBWhpszLGwZ` | outputAmount: 79681 USDC micro; confirmed on-chain |
| 9 | [Regression] L2 re-run after --force fix | L2 | `raydium get-token-price --mints EPjFWdd5...` | PASS | USDC = $1.00 | No regression |
| 10 | [Regression] L3 re-run after --force fix | L3 | `raydium swap --dry-run --input-mint So11... --output-mint EPjFWdd5... --amount 1000000` | PASS | `ok: true`, `dry_run: true` | No regression |
| 11 | [Regression] L4 re-run after --force fix | L4 | `raydium swap --input-mint So11... --output-mint EPjFWdd5... --amount 1000000` | PASS | `4gDhPAuYmc4Htvs1M8Cabest8Xnf9pqgMPYFXDUVRBgqAiFeAHeUpeWgyPUvURj5SgSiKMpd29CHa5VHTAF2ZwQf` | outputAmount: 79608 USDC micro; confirmed on-chain with --force |

## Fix Records

| # | Issue | Root Cause | Fix | File |
|---|-------|-----------|-----|------|
| 1 | `--output json` fails for Solana chain 501 | `onchainos wallet balance --chain 501 --output json` returns empty; Solana returns JSON natively | Remove `--output json` flag | `src/onchainos.rs` |
| 2 | `--unsigned-tx` missing `--to` parameter | `onchainos wallet contract-call` requires `--to <program>` even for Solana | Add `--to 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` (Raydium AMM V4) | `src/onchainos.rs` |
| 3 | `computeUnitPriceMicroLamports: "auto"` rejected by Raydium API | API requires numeric string, not literal "auto" | Changed default to `"1000"` | `src/commands/swap.rs` |
| 4 | `--unsigned-tx` expects base58 but Raydium API returns base64 | onchainos help: "Solana unsigned transaction data (base58)" | Added base64→base58 conversion in `wallet_contract_call_solana()` | `src/onchainos.rs` |
| 5 | `wallet contract-call` requires `--force` to broadcast | onchainos will not broadcast Solana contract-call without explicit `--force` flag (discovered from Kamino retro) | Added `"--force"` to wallet_contract_call_solana args | `src/onchainos.rs` |

## Notes

### Wallet
- Solana address: `6hY15MNMZtjF15sPtuSozxjrrZPyrDmqBaC48496T8UY`
- SOL spent: ~0.001 SOL (swap) + gas fees
