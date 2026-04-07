---
name: compound-v2
description: "Compound V2 classic cToken lending: supply assets to earn interest, redeem cTokens, view positions, borrow (dry-run), repay (dry-run), claim COMP rewards. Trigger phrases: compound supply, compound lend, compound redeem, compound borrow, compound repay, compound positions, compound markets, claim COMP, cToken, 在Compound供应, Compound存款, Compound借款, Compound仓位, 领取COMP"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## ⚠️ Protocol Status: Deprecated

Compound V2 has been **officially deprecated** by Compound governance. All supply/borrow reserves are **frozen** — new deposits (`mint`) and new borrows are rejected at the contract level with "mint is paused".

**What still works:**
- ✅ `markets` / `positions` — read market data and view existing positions
- ✅ `redeem` — existing suppliers can withdraw their funds
- ✅ `claim-comp` — claim accrued COMP rewards
- ❌ `supply` — will fail on-chain (reserves frozen)
- ❌ `borrow` / `repay` — dry-run only regardless

**Recommendation:** For active lending/borrowing, use **Compound V3** (`compound-v3`) instead, which is the actively maintained successor.

---

## Architecture

- Read ops (`markets`, `positions`) → direct `eth_call` via public RPC; no wallet needed
- Write ops (`supply`, `redeem`, `claim-comp`) → after user confirmation, submits via `onchainos wallet contract-call --force`
- Dry-run only (`borrow`, `repay`) → always returns preview; never broadcasts

## Supported Chain

| Chain | Chain ID | Protocol |
|-------|----------|---------|
| Ethereum Mainnet | 1 | Compound V2 (cToken) |

## Supported Assets

| Symbol | cToken | Underlying |
|--------|--------|-----------|
| ETH | cETH `0x4Ddc2D193948926D02f9B1fE9e1daa0718270ED5` | Native ETH |
| USDT | cUSDT `0xf650C3d88D12dB855b8bf7D11Be6C55A4e07dCC9` | `0xdAC17F958D2ee523a2206206994597C13D831ec7` |
| USDC | cUSDC `0x39AA39c021dfbaE8faC545936693aC917d5E7563` | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` |
| DAI | cDAI `0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643` | `0x6B175474E89094C44Da98b954EedeAC495271d0F` |

## Commands

### markets — List cToken markets

```bash
compound-v2 [--chain 1] markets
```

Returns supply APR, borrow APR, and exchange rate for each cToken market. Read-only; no wallet needed.

---

### positions — View your positions

```bash
compound-v2 [--chain 1] positions [--wallet 0x...]
```

Returns supplied (cToken balance + underlying equivalent) and borrowed amounts per market. Read-only.

---

### supply — Supply an asset to earn interest

```bash
# Preview (dry-run)
compound-v2 --chain 1 --dry-run supply --asset USDT --amount 0.01

# Execute
compound-v2 --chain 1 supply --asset USDT --amount 0.01 --from 0xYourWallet
```

**Execution flow:**
1. Run with `--dry-run` to preview the steps and calldata
2. **Ask user to confirm** the asset, amount, and that they will receive cTokens in return
3. For ERC20 assets: execute `ERC20.approve(cToken, amount)`, wait 3 seconds, then `cToken.mint(amount)`
4. For ETH: execute `cETH.mint()` as a payable call with ETH value
5. Report approve txHash (ERC20 only), mint txHash, and updated cToken balance

---

### redeem — Redeem cTokens to get back underlying

```bash
# Preview (dry-run)
compound-v2 --chain 1 --dry-run redeem --asset USDT --ctoken-amount 0.5

# Execute
compound-v2 --chain 1 redeem --asset USDT --ctoken-amount 0.5 --from 0xYourWallet
```

**Execution flow:**
1. Run with `--dry-run` to preview
2. **Ask user to confirm** the amount of cTokens to burn and underlying to receive
3. Check cToken balance — fail if insufficient
4. Execute `cToken.redeem(cTokenAmount)`
5. Report txHash and updated cToken balance

---

### borrow — Preview borrowing (DRY-RUN ONLY)

```bash
compound-v2 --chain 1 --dry-run borrow --asset USDT --amount 1.0
```

**Note:** Borrow is dry-run only for safety. Shows the calldata and steps. Requires collateral to be supplied first on Compound V2. Never executes on-chain.

---

### repay — Preview repaying borrow (DRY-RUN ONLY)

```bash
compound-v2 --chain 1 --dry-run repay --asset USDT --amount 1.0
```

**Note:** Repay is dry-run only for safety. Shows approve + repayBorrow steps. Never executes on-chain.

---

### claim-comp — Claim COMP governance rewards

```bash
# Preview (dry-run)
compound-v2 --chain 1 --dry-run claim-comp

# Execute
compound-v2 --chain 1 claim-comp --from 0xYourWallet
```

**Execution flow:**
1. Run with `--dry-run` to preview
2. **Ask user to confirm** before claiming
3. Execute `Comptroller.claimComp(wallet)`
4. Report txHash

---

## Key Concepts

**cTokens represent your supply position**
When you supply assets, you receive cTokens. The exchange rate increases over time as interest accrues. To get your assets back, redeem cTokens.

**Exchange rate**
`underlying = cToken_balance × exchangeRate / 1e18`
The exchange rate starts at ~0.02 and grows monotonically.

**Borrow requires collateral**
To borrow, you must first supply collateral. Each asset has a collateral factor (e.g., 75% for ETH). Your total borrow must not exceed your borrowing capacity.

**COMP rewards**
Compound V2 distributes COMP tokens to suppliers and borrowers. Use `claim-comp` to collect accrued rewards.

## Dry-Run Mode

All write operations support `--dry-run`. In dry-run mode:
- No transactions are broadcast
- Returns expected calldata, steps, and amounts as JSON
- Use to preview before asking for user confirmation

## Error Responses

All commands return structured JSON:
```json
{"ok": false, "error": "human-readable error message"}
```
