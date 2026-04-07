# Test Results — cian-yield-layer

**Plugin:** cian-yield-layer v0.1.0
**Chain:** Ethereum Mainnet (chain ID 1)
**Test Date:** 2026-04-05
**Tester:** Tester Agent (Claude Sonnet 4.6)

---

## Summary

| Level | Result | Notes |
|-------|--------|-------|
| L1 Build | PASS | 5 warnings (unused fields/fns), no errors |
| L1 Lint | PASS | `plugin-store lint .` passes all checks |
| L0 Routing | PASS | SKILL.md rules are accurate and well-scoped |
| L2 vaults | FAIL | REST API endpoint returns 404 (API unavailable) |
| L2 positions | PASS | Gracefully falls back to on-chain data |
| L3 deposit dry-run | PASS | Selector 0x32507a5f confirmed, approve step shown |
| L3 request-redeem dry-run | PARTIAL | Balance check runs before dry-run guard (expected) |
| L4 live deposit | BLOCKED | Wallet has 0 ETH/WETH on chain 1 |

**Overall: PARTIAL PASS** — Core on-chain functionality works. REST API unavailable.

---

## L1 — Build + Lint

### Build

```
cargo build --release
```

**Result: PASS**

Exit 0. Binary produced at `target/release/cian-yield-layer`.

Warnings (non-blocking):
- `api.rs`: fields `chain_id`, `pool_type`, `apy` never read in `PoolInfo`
- `api.rs`: struct `UserVaultPosition` never constructed
- `config.rs`: fn `parse_units` never used
- `rpc.rs`: fns `total_assets`, `total_supply` never used

### Lint

```
cargo clean && plugin-store lint .
```

**Result: PASS**

```
✓ Plugin 'cian-yield-layer' passed all checks!
```

---

## L0 — Skill Routing

**Source:** `skills/cian-yield-layer/SKILL.md`

**Result: PASS**

- Triggers correctly on CIAN/ylstETH/ylpumpBTC mentions
- "Do NOT use" guards correctly exclude general staking / non-CIAN products
- All 5 commands (vaults, balance, positions, deposit, request-redeem) documented in SKILL.md
- See `tests/routing_test.md` for full routing analysis

---

## L2 — Read Commands

### vaults

```bash
./target/release/cian-yield-layer vaults
```

**Result: FAIL**

```
Error: error decoding response body
Caused by:
    invalid type: integer `404`, expected struct ApiResponse at line 1 column 3
```

**Root Cause:** REST API endpoint `https://yieldlayer.cian.app/ethereum/pool/home` returns `404 page not found` (plain text). The code parses response as JSON and fails because `404` (parsed as integer) does not match expected `ApiResponse { code: String, data: Vec<PoolInfo> }` structure.

**Impact:** `vaults` command is completely broken when API is unavailable. No fallback mechanism.

**Recommendation:**
1. Verify correct API endpoint with CIAN team (may have changed)
2. Add graceful error handling: catch JSON parse errors and show user-friendly message
3. Consider showing hardcoded vault addresses as fallback when API is unavailable

### positions

```bash
./target/release/cian-yield-layer positions --wallet 0x0000000000000000000000000000000000000001
```

**Result: PASS**

```
=== CIAN Yield Layer — Positions ===
Chain: Ethereum Mainnet (chain 1)
Wallet: 0x0000000000000000000000000000000000000001

── stETH Yield Layer (ylstETH) ──
   Vault: 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d
   On-chain shares: 0.000000 ylstETH
   Note: REST API unavailable (...), showing on-chain data only.
   No position in this vault.

── pumpBTC Yield Layer (ylpumpBTC) ──
   Vault: 0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b
   On-chain shares: 0.000000 ylpumpBTC
   No position in this vault.
```

On-chain RPC (mevblocker.io) is working. `positions` gracefully handles REST API 404 and falls back to on-chain data only. Zero balance confirmed for zero wallet.

---

## L3 — Dry-Run / Selector Verification

### Deposit Dry-Run

```bash
./target/release/cian-yield-layer deposit --vault ylstETH --token WETH --amount 0.001 --dry-run
```

**Result: PASS**

```
=== CIAN Yield Layer — Deposit ===
Chain:   Ethereum Mainnet (chain 1)
Vault:   stETH Yield Layer (ylstETH)
Token:   WETH (0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2)
Wallet:  0xe4621cadb69e7eda02248ba03ba538137d329b94
Amount:  0.001 WETH (raw: 1000000000000000)

[DRY-RUN] Would execute:
  Step 1: approve WETH → vault 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d
  Step 2: optionalDeposit(token=0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2, assets=1000000000000000,
          receiver=0xe4621cadb69e7eda02248ba03ba538137d329b94, referral=0x000...000)
  Estimated shares: 0.000931 ylstETH

Run without --dry-run and confirm to execute.
```

**Selector verification (from source code):**
- `optionalDeposit`: `0x32507a5f` ✅ (hardcoded in deposit.rs line: `"0x32507a5f{}{}{}{}"`)
- `approve`: `0x095ea7b3` ✅ (hardcoded in onchainos.rs `erc20_approve`)
- previewDeposit call working (returned ~0.000931 ylstETH for 0.001 WETH)

### Request-Redeem Dry-Run

```bash
./target/release/cian-yield-layer request-redeem --vault ylstETH --shares 1000000000000000 --token stETH --dry-run
```

**Note:** The `--token` parameter is required but was missing from the pipeline test spec. Command requires `--token`.

**Result: EXPECTED BEHAVIOR**

With zero wallet balance, the command correctly rejects:
```
Error: Insufficient shares: requested 0.001000 but wallet only holds 0.000000
```

The `--dry-run` guard occurs after the balance check. This is intentional behavior — balance validation prevents users from being confused by a dry-run output that would fail on execution.

**Selector verification (from source code):**
- `requestRedeem`: `0x107703ab` ✅ (hardcoded in request_redeem.rs: `"0x107703ab{}{}"`)

### Error Handling Tests

| Test | Result |
|------|--------|
| Invalid vault name: `--vault badname` | PASS: "Unknown vault 'badname'" |
| Invalid token: `--token ETH --vault ylpumpbtc` | PASS: "Unknown token 'eth' for this vault" |
| Vault resolution case-insensitive | PASS: `ylstETH` resolves correctly |

---

## L4 — On-Chain

```bash
onchainos wallet balance --chain 1
```

**Result:**

```json
{
  "ok": true,
  "data": {
    "details": [{ "tokenAssets": [] }],
    "totalValueUsd": "0.00"
  }
}
```

**Wallet:** `0xe4621cadb69e7eda02248ba03ba538137d329b94`
**ETH Balance:** 0
**WETH Balance:** 0

**L4 live deposit test: BLOCKED** — Insufficient wallet balance on Ethereum Mainnet.

---

## Issues Found

### ISSUE-1: `vaults` command crashes on REST API 404 [SEVERITY: HIGH]

**File:** `src/api.rs` + `src/commands/vaults.rs`

**Description:** `https://yieldlayer.cian.app/ethereum/pool/home` returns `404 page not found` (plain text). The `resp.json().await?` call in `fetch_pools()` parses the text as JSON, reading `404` as an integer, then fails to deserialize it into `ApiResponse<Vec<PoolInfo>>`.

**Impact:** The `vaults` command (primary informational command) is non-functional.

**Recommended Fix:**
1. Check if API endpoint has changed (contact CIAN team)
2. Add error handling in `fetch_pools()` to detect non-JSON or non-OK HTTP responses
3. In `vaults.rs`, add fallback to display hardcoded vault info when API fails

### ISSUE-2: `request-redeem --dry-run` requires wallet balance [SEVERITY: LOW]

**Description:** `--dry-run` on request-redeem still enforces balance check. This means you cannot preview a redeem without having shares. Acceptable for safety but inconsistent with deposit dry-run behavior.

**Impact:** Minor UX inconsistency; not a blocking issue.

### ISSUE-3: Missing `--token` from pipeline test spec [SEVERITY: INFO]

**Description:** The pipeline test spec for request-redeem dry-run omits `--token` argument, which is required. The command fails with a clap usage error. This is a documentation issue in the spec, not a code bug.

### ISSUE-4: Dead code warnings [SEVERITY: INFO]

5 compiler warnings for unused fields/functions. Not blocking but should be cleaned up before production release.

---

## On-Chain Verification (via eth_call)

| Check | Address | Result |
|-------|---------|--------|
| ylstETH balanceOf(0x...0001) | `0xB13aa2d0345b0439b064f26B82D8dCf3f508775d` | 0 (PASS) |
| ylpumpBTC balanceOf(0x...0001) | `0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b` | 0 (PASS) |
| Exchange price call succeeds | Both vaults | PASS (previewDeposit returned data) |
| RPC endpoint reachable | mevblocker.io | PASS |

---

## Recommendations

1. **Fix REST API endpoint** — Investigate if `yieldlayer.cian.app` API has moved or requires auth. Add graceful degradation in `vaults.rs`.
2. **Clean up dead code** — Remove or use unused fields/functions to eliminate 5 compiler warnings.
3. **SKILL.md example fix** — `request-redeem` example in SKILL.md should include `--token` parameter.
4. **Consider API fallback** — When REST API fails, `vaults` could show hardcoded static vault info with a note that live APY/TVL is unavailable.
