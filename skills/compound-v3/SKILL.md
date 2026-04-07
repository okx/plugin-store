---
name: compound-v3
description: "Compound V3 (Comet) lending plugin: supply collateral, borrow/repay the base asset, and claim COMP rewards. Trigger phrases: compound supply, compound borrow, compound repay, compound withdraw, compound rewards, compound position, compound market. Chinese: 在Compound供应, 从Compound借款, 还款Compound, 提取Compound抵押品, 领取COMP奖励"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

## Do NOT use for

Do NOT use for Aave, Morpho, Euler, Spark, or other lending protocols.
Do NOT use for token swaps — use a DEX plugin instead.
Do NOT use for Compound V2 — use compound-v2 plugin instead.

## Architecture

- Read ops (`get-markets`, `get-position`) → direct `eth_call` via public RPC; no confirmation needed
- Write ops (`supply`, `borrow`, `withdraw`, `repay`, `claim-rewards`) → after user confirmation, submits via `onchainos wallet contract-call`

## Supported Chains and Markets

| Chain | Chain ID | Market | Comet Proxy |
|-------|----------|--------|-------------|
| Ethereum | 1 | usdc | 0xc3d688B66703497DAA19211EEdff47f25384cdc3 |
| Base | 8453 | usdc | 0xb125E6687d4313864e53df431d5425969c15Eb2F |
| Arbitrum | 42161 | usdc | 0x9c4ec768c28520B50860ea7a15bd7213a9fF58bf |
| Polygon | 137 | usdc | 0xF25212E676D1F7F89Cd72fFEe66158f541246445 |

Default chain: Base (8453). Default market: usdc.

## Commands

### get-markets — View market statistics

```bash
compound-v3 [--chain 8453] [--market usdc] get-markets
```

Reads utilization, supply APR, borrow APR, total supply, and total borrow directly from the Comet contract. No wallet needed.

---

### get-position — View account position

```bash
compound-v3 [--chain 8453] [--market usdc] get-position [--wallet 0x...] [--collateral-asset 0x...]
```

Returns supply balance, borrow balance, and whether the account is collateralized. Read-only; no confirmation needed.

---

### supply — Supply collateral or base asset

Supplying base asset (e.g. USDC) when debt exists will automatically repay debt first.

```bash
# Preview (dry-run)
compound-v3 --chain 8453 --market usdc --dry-run supply \
  --asset 0x4200000000000000000000000000000000000006 \
  --amount 100000000000000000

# Execute
compound-v3 --chain 8453 --market usdc supply \
  --asset 0x4200000000000000000000000000000000000006 \
  --amount 100000000000000000 \
  --from 0xYourWallet
```

**Execution flow:**
1. Run with `--dry-run` to preview the approve + supply steps
2. **Ask user to confirm** the supply amount, asset, and market before proceeding
3. Execute ERC-20 approve: `onchainos wallet contract-call` → token.approve(comet, amount)
4. Wait 3 seconds (nonce safety)
5. Execute supply: `onchainos wallet contract-call` → Comet.supply(asset, amount)
6. Report approve txHash, supply txHash, and updated supply balance

---

### borrow — Borrow base asset

Borrow is implemented as `Comet.withdraw(base_asset, amount)`. No ERC-20 approve required. Collateral must be supplied first.

```bash
# Preview (dry-run)
compound-v3 --chain 8453 --market usdc --dry-run borrow --amount 100000000

# Execute
compound-v3 --chain 8453 --market usdc borrow --amount 100000000 --from 0xYourWallet
```

**Execution flow:**
1. Pre-check: `isBorrowCollateralized` must be true; amount must be ≥ `baseBorrowMin`
2. Run with `--dry-run` to preview
3. **Ask user to confirm** the borrow amount and ensure they understand debt accrues interest
4. Execute: `onchainos wallet contract-call` → Comet.withdraw(base_asset, amount)
5. Report txHash and updated borrow balance

---

### repay — Repay borrowed base asset

Repay uses `Comet.supply(base_asset, amount)`. The plugin reads `borrowBalanceOf` and uses `min(borrow, wallet_balance)` to avoid overflow revert.

```bash
# Preview repay-all (dry-run)
compound-v3 --chain 8453 --market usdc --dry-run repay

# Execute repay-all
compound-v3 --chain 8453 --market usdc repay --from 0xYourWallet

# Execute partial repay
compound-v3 --chain 8453 --market usdc repay --amount 50000000 --from 0xYourWallet
```

**Execution flow:**
1. Read current `borrowBalanceOf` and wallet token balance
2. Run with `--dry-run` to preview
3. **Ask user to confirm** the repay amount before proceeding
4. Execute ERC-20 approve: `onchainos wallet contract-call` → token.approve(comet, amount)
5. Wait 3 seconds
6. Execute repay: `onchainos wallet contract-call` → Comet.supply(base_asset, repay_amount)
7. Report approve txHash, repay txHash, and remaining debt

---

### withdraw — Withdraw supplied collateral

Withdraw requires zero outstanding debt. The plugin enforces this with a pre-check.

```bash
# Preview (dry-run)
compound-v3 --chain 8453 --market usdc --dry-run withdraw \
  --asset 0x4200000000000000000000000000000000000006 \
  --amount 100000000000000000

# Execute
compound-v3 --chain 8453 --market usdc withdraw \
  --asset 0x4200000000000000000000000000000000000006 \
  --amount 100000000000000000 \
  --from 0xYourWallet
```

**Execution flow:**
1. Pre-check: `borrowBalanceOf` must be 0. If debt exists, prompt user to repay first.
2. Run with `--dry-run` to preview
3. **Ask user to confirm** the withdrawal before proceeding
4. Execute: `onchainos wallet contract-call` → Comet.withdraw(asset, amount)
5. Report txHash

---

### claim-rewards — Claim COMP rewards

Rewards are claimed via the CometRewards contract. The plugin checks `getRewardOwed` first — if zero, it returns a friendly message without submitting any transaction.

```bash
# Preview (dry-run)
compound-v3 --chain 1 --market usdc --dry-run claim-rewards

# Execute
compound-v3 --chain 1 --market usdc claim-rewards --from 0xYourWallet
```

**Execution flow:**
1. Pre-check: call `CometRewards.getRewardOwed(comet, wallet)`. If 0, return "No claimable rewards."
2. Show reward amount to user
3. **Ask user to confirm** before claiming
4. Execute: `onchainos wallet contract-call` → CometRewards.claimTo(comet, wallet, wallet, true)
5. Report txHash and confirmation

---

## Key Concepts

**supply = repay when debt exists**
Supplying the base asset (e.g. USDC) automatically repays any outstanding debt first. The plugin always shows current borrow balance and explains this behavior.

**borrow = withdraw base asset**
In Compound V3, `Comet.withdraw(base_asset, amount)` creates a borrow position when there is insufficient supply balance. The plugin distinguishes borrow from regular withdraw by checking `borrowBalanceOf`.

**repay overflow protection**
Never use `uint256.max` for repay. The plugin reads `borrowBalanceOf` and uses `min(borrow_balance, wallet_balance)` to prevent revert when accrued interest exceeds wallet balance.

**withdraw requires zero debt**
Attempting to withdraw collateral while in debt will revert. The plugin checks `borrowBalanceOf` and blocks the withdraw with a clear error message if debt is outstanding.

## Dry-Run Mode

All write operations support `--dry-run`. In dry-run mode:
- No transactions are submitted
- The expected calldata, steps, and amounts are returned as JSON
- Use this to preview before asking for user confirmation

## Error Responses

All commands return structured JSON. On error:
```json
{"ok": false, "error": "human-readable error message"}
```
