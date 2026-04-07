# Test Cases — cian-yield-layer

**Plugin:** cian-yield-layer v0.1.0
**Chain:** Ethereum Mainnet (chain ID 1)
**Test Date:** 2026-04-05

---

## Vaults Under Test

| Vault | Symbol | Address |
|-------|--------|---------|
| stETH Yield Layer | ylstETH | `0xB13aa2d0345b0439b064f26B82D8dCf3f508775d` |
| pumpBTC Yield Layer | ylpumpBTC | `0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b` |

---

## L1 — Compile + Lint

| ID | Test | Expected |
|----|------|----------|
| L1-01 | `cargo build --release` | Exit 0, binary produced |
| L1-02 | `cargo clean && plugin-store lint .` | No errors |

---

## L0 — Skill Routing

| ID | User Utterance | Expected Skill Trigger |
|----|---------------|----------------------|
| R-01 | "CIAN vaults" | YES |
| R-02 | "ylstETH balance" | YES |
| R-03 | "deposit stETH CIAN" | YES |
| R-04 | "CIAN withdraw" | YES |
| R-05 | "CIAN APY" | YES |
| R-06 | "stake ETH CIAN yield" | YES |
| R-07 | "stake ETH on Lido" | NO (not CIAN-specific) |
| R-08 | "swap ETH for USDC" | NO |

---

## L2 — Read-Only Commands

| ID | Command | Expected Output |
|----|---------|----------------|
| R2-01 | `./target/release/cian-yield-layer vaults` | Lists ylstETH and ylpumpBTC with APY/TVL from REST API |
| R2-02 | Vault list contains ylstETH address | `0xB13aa2d0345b0439b064f26B82D8dCf3f508775d` |
| R2-03 | Vault list contains ylpumpBTC address | `0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b` |
| R2-04 | `positions --wallet 0x0000...0001` | Shows zero balance; does not crash |
| R2-05 | `balance --wallet 0x0000...0001` | Shows zero shares across vaults |

---

## L3 — Dry-Run / Selector Verification

| ID | Command | Expected |
|----|---------|----------|
| D-01 | `deposit --vault ylstETH --token WETH --amount 0.001 --dry-run` | Prints DRY-RUN, calldata starts with `0x32507a5f` |
| D-02 | deposit dry-run includes approve step | Shows "Step 1: approve WETH → vault" |
| D-03 | deposit dry-run shows estimated shares | Prints "Estimated shares: X.XXXXXX ylstETH" |
| D-04 | `request-redeem --vault ylstETH --shares 1000000000000000 --dry-run` | Prints DRY-RUN, mentions `requestRedeem(shares=..., token=...)` |
| D-05 | request-redeem dry-run with zero wallet balance | Fails with "Insufficient shares" or warns 0 balance |
| D-06 | approve calldata selector | `0x095ea7b3` |
| D-07 | Invalid vault name rejected | Error: "Unknown vault" |
| D-08 | Invalid token for vault rejected | Error: "Unknown token" |

---

## L4 — On-Chain (Live)

| ID | Test | Expected |
|----|------|----------|
| OC-01 | `onchainos wallet balance --chain 1` | Returns wallet address + balance |
| OC-02 | If ETH/WETH balance > 0.001 ETH: `deposit --vault ylstETH --token WETH --amount 0.001` | TX hash returned, ylstETH shares increase |
| OC-03 | If no balance: document BLOCKED | Skip live deposit test |

---

## Key Function Selectors (Verified Against Etherscan)

| Function | Selector |
|----------|---------|
| `optionalDeposit(address,uint256,address,address)` | `0x32507a5f` |
| `requestRedeem(uint256,address)` | `0x107703ab` |
| `approve(address,uint256)` | `0x095ea7b3` |
| `balanceOf(address)` | `0x70a08231` |
| `convertToAssets(uint256)` | `0x07a2d13a` |
