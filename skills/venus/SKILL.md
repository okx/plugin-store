---
name: venus
description: "Venus Core Pool lending and borrowing on BSC. Supply assets, borrow, repay, withdraw, and manage collateral on the Venus Compound V2 fork. Trigger phrases: supply to venus, borrow from venus, venus positions, check venus markets, repay venus loan, withdraw from venus, claim venus rewards, venus APY, venus lending, venus BSC"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Venus Core Pool

Venus is a Compound V2 fork on BNB Smart Chain (BSC, chain 56). It allows users to supply assets to earn interest (via vTokens) and borrow against collateral.

## Architecture

- Read ops (get-markets, get-positions) use direct `eth_call` via public BSC RPC; no confirmation needed
- Write ops (supply, withdraw, borrow, repay, enter-market, claim-rewards) submit via `onchainos wallet contract-call` after user confirmation
- All on-chain operations target BSC (chain ID 56)

## Execution Flow for Write Operations

1. Run with `--dry-run` to preview calldata and parameters
2. **Ask user to confirm** the transaction details before executing on-chain
3. Execute only after explicit user approval
4. Report transaction hash and outcome

---

## Commands

### get-markets

List all Venus Core Pool markets with supply/borrow APY and utilization.

**Usage:**
```
venus get-markets --chain 56
```

**Example output:**
```json
{
  "ok": true,
  "chain_id": 56,
  "market_count": 48,
  "markets": [
    {
      "symbol": "vUSDT",
      "underlying_symbol": "USDT",
      "supply_apy_pct": "2.4500",
      "borrow_apy_pct": "3.8200",
      "total_borrows_raw": "...",
      "total_cash_raw": "..."
    }
  ]
}
```

---

### get-positions

Show your current supply and borrow positions across all Venus markets.

**Usage:**
```
venus get-positions --chain 56
venus get-positions --chain 56 --wallet 0xYourAddress
```

---

### supply

Supply an asset to Venus to earn interest. Receives vTokens in return.

**Supported assets:** BNB, USDT, BTC, ETH, USDC

**Usage:**
```
venus supply --asset USDT --amount 10.0 --chain 56 --dry-run
venus supply --asset BNB --amount 0.01 --chain 56 --dry-run
```

**Before executing:**
- Run with `--dry-run` to preview the transaction
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- ERC-20 assets: calls `approve(vToken, amount)` then `vToken.mint(amount)` via `onchainos wallet contract-call`
- Native BNB: calls `vBNB.mint()` with `--amt <wei>` via `onchainos wallet contract-call`

---

### withdraw

Withdraw a previously supplied asset (redeem underlying).

**Usage:**
```
venus withdraw --asset USDT --amount 5.0 --chain 56 --dry-run
```

**Before executing:**
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- Calls `vToken.redeemUnderlying(amount)` via `onchainos wallet contract-call`

---

### borrow

Borrow an asset against your supplied collateral. Requires collateral to be enabled via `enter-market` first.

**Usage:**
```
venus borrow --asset USDT --amount 5.0 --chain 56 --dry-run
```

**Before executing:**
- Ensure you have supplied collateral and entered the market
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- Calls `vToken.borrow(amount)` via `onchainos wallet contract-call`

---

### repay

Repay borrowed assets to Venus.

**Usage:**
```
venus repay --asset USDT --amount 5.0 --chain 56 --dry-run
```

**Before executing:**
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- ERC-20: calls `approve(vToken, amount)` then `vToken.repayBorrow(amount)` via `onchainos wallet contract-call`

---

### enter-market

Enable an asset as collateral so it can be used to back borrowing positions.

**Usage:**
```
venus enter-market --asset USDT --chain 56 --dry-run
```

**Before executing:**
- Asset must already be supplied
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- Calls `Comptroller.enterMarkets([vToken])` via `onchainos wallet contract-call`

---

### claim-rewards

Claim accrued XVS token rewards from the Venus Comptroller.

**Usage:**
```
venus claim-rewards --chain 56 --dry-run
venus claim-rewards --chain 56 --wallet 0xYourAddress --dry-run
```

**Before executing:**
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- Calls `Comptroller.claimVenus(holder)` via `onchainos wallet contract-call`

---

## Key Contracts (BSC mainnet, chain 56)

| Contract | Address |
|----------|---------|
| Comptroller | `0xfD36E2c2a6789Db23113685031d7F16329158384` |
| vBNB | `0xa07c5b74c9b40447a954e1466938b865b6bbea36` |
| vUSDT | `0xfd5840cd36d94d7229439859c0112a4185bc0255` |
| vBTC | `0x882c173bc7ff3b7786ca16dfed3dfffb9ee7847b` |
| vETH | `0xf508fcd89b8bd15579dc79a6827cb4686a3592c8` |
| vUSDC | `0xeca88125a5adbe82614ffc12d0db554e2e2867c8` |

## Notes

- Venus is a Compound V2 fork on BNB Smart Chain (BSC)
- All operations are on chain ID 56
- Interest is calculated per BSC block (approximately every 3 seconds)
- vTokens represent your share in the supply pool; their exchange rate increases over time
- Always supply and enter a market before attempting to borrow
- Repay borrowings before attempting full withdrawal to avoid health factor revert
