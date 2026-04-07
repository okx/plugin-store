---
name: segment-finance
description: "Segment Finance lending and borrowing on BNB Chain (BSC). Supply assets to earn interest, borrow against collateral, repay, and withdraw on this Compound V2 fork. Trigger phrases: supply to segment, borrow from segment, segment finance positions, check segment markets, repay segment loan, withdraw from segment finance, segment finance APY, segment lending BSC, segment finance BNB"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Segment Finance

Segment Finance is a Compound V2 fork on BNB Smart Chain (BSC, chain 56). It allows users to supply assets to earn interest (via seTokens) and borrow against collateral.

## Architecture

- Read ops (get-markets, get-positions) use direct eth_call via public BSC RPC; no confirmation needed
- Write ops (supply, withdraw, borrow, repay, enter-market) submit via `onchainos wallet contract-call` after user confirmation
- All on-chain operations target BSC (chain ID 56)
- Comptroller uses Diamond proxy pattern (EIP-2535)

## Execution Flow for Write Operations

1. Run with `--dry-run` to preview calldata and parameters
2. **Ask user to confirm** the transaction details before executing on-chain
3. Execute only after explicit user approval
4. Report transaction hash and outcome

---

## Commands

### get-markets

List all Segment Finance markets with supply/borrow APY, utilization, and USD prices.

**Usage:**
```
segment-finance get-markets --chain 56
```

**Example output:**
```json
{
  "ok": true,
  "chain_id": 56,
  "protocol": "Segment Finance",
  "market_count": 5,
  "markets": [
    {
      "symbol": "seUSDT",
      "underlying_symbol": "USDT",
      "supply_apy_pct": "2.4500",
      "borrow_apy_pct": "3.8200",
      "price_usd": "1.0000"
    }
  ]
}
```

---

### get-positions

Show your current supply and borrow positions across all Segment Finance markets.

**Usage:**
```
segment-finance get-positions --chain 56
segment-finance get-positions --chain 56 --wallet 0xYourAddress
```

---

### supply

Supply an asset to Segment Finance to earn interest. Receives seTokens in return.

**Supported assets:** BNB, USDT, USDC, BTC, ETH

**Usage:**
```
segment-finance supply --asset USDT --amount 10.0 --chain 56 --dry-run
segment-finance supply --asset BNB --amount 0.01 --chain 56 --dry-run
```

**Before executing:**
- Run with `--dry-run` to preview the transaction
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- ERC-20 assets: calls `approve(seToken, amount)` then `seToken.mint(amount)` via `onchainos wallet contract-call`
- Native BNB: calls `seBNB.mint()` with `--amt <wei>` via `onchainos wallet contract-call`

---

### withdraw

Withdraw a previously supplied asset (redeem underlying).

**Usage:**
```
segment-finance withdraw --asset USDT --amount 5.0 --chain 56 --dry-run
```

**Before executing:**
- Ensure all borrowed debt is repaid before full withdrawal
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- Calls `seToken.redeemUnderlying(amount)` via `onchainos wallet contract-call`

---

### borrow

Borrow an asset against your supplied collateral. Requires collateral to be enabled via `enter-market` first.

**Usage:**
```
segment-finance borrow --asset USDT --amount 5.0 --chain 56 --dry-run
```

**Before executing:**
- Ensure you have supplied collateral and entered the market
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- Calls `seToken.borrow(amount)` via `onchainos wallet contract-call`

---

### repay

Repay borrowed assets to Segment Finance.

**Usage:**
```
segment-finance repay --asset USDT --amount 5.0 --chain 56 --dry-run
```

**Before executing:**
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- ERC-20: calls `approve(seToken, amount)` then `seToken.repayBorrow(amount)` via `onchainos wallet contract-call`

---

### enter-market

Enable an asset as collateral so it can be used to back borrowing positions.

**Usage:**
```
segment-finance enter-market --asset USDT --chain 56 --dry-run
```

**Before executing:**
- Asset must already be supplied (have seToken balance)
- **Ask user to confirm** before submitting on-chain

**On-chain execution (after confirmation):**
- Calls `Comptroller.enterMarkets([seToken])` via `onchainos wallet contract-call`

---

## Key Contracts (BSC mainnet, chain 56)

| Contract | Address |
|----------|---------|
| Comptroller (Unitroller) | `0x57E09c96DAEE58B77dc771B017de015C38060173` |
| Oracle | `0x763217cFeFac3B26191b1DCaE1926F65157B9A05` |
| seBNB | `0x5fceA94B96858048433359BB5278a402363328C3` |
| seUSDT | `0x44B1E0f4533FD155B9859a9DB292C90E5B300119` |
| seUSDC | `0x8969b89D5f38359fBE95Bbe392f5ad82dd93e226` |
| seBTC | `0x12CD46B96fe0D86E396248a623B81fD84dD0F61d` |
| seETH | `0x3821175E59CD0acDa6c5Fd3eBB618b204e5D7eed` |

## Notes

- Segment Finance is a Compound V2 fork on BNB Smart Chain (BSC, chain 56)
- The Comptroller uses Diamond proxy (EIP-2535) pattern
- BSC USDT has 18 decimals (unlike Ethereum USDT which has 6)
- seTokens represent your share in the supply pool; exchange rate increases over time
- Always supply and enter a market before attempting to borrow
- Repay all borrowings before attempting full withdrawal to maintain healthy collateral ratio
- Protocol also supports BOB, opBNB, CORE networks but this plugin targets BSC only
