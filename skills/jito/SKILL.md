---
name: jito
version: 0.1.0
description: Jito MEV-enhanced liquid staking on Solana — stake SOL, earn JitoSOL rewards
chains:
  - solana
category: defi-protocol
tags:
  - staking
  - liquid-staking
  - solana
  - jitosol
  - mev
---

# Jito Liquid Staking Skill

Jito is a MEV-enhanced liquid staking protocol on Solana. Stake SOL to receive JitoSOL, which automatically earns staking rewards plus MEV rewards from Jito's block engine.

## Binary

`jito` — Rust binary that interacts with the Jito SPL Stake Pool on Solana.

## Commands

### rates — Query Staking Rates

Query the current SOL ↔ JitoSOL exchange rate and estimated APY.

**Trigger phrases:**
- "What's the Jito staking APY?"
- "Check Jito JitoSOL rate"
- "How much JitoSOL do I get for 1 SOL?"
- "Jito staking yield"

**Usage:**
```bash
jito rates --chain 501
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "protocol": "Jito",
    "chain": "Solana",
    "sol_per_jitosol": "1.27127624",
    "jitosol_per_sol": "0.78661110",
    "total_staked_sol": "11227420.1819",
    "total_jitosol_supply": "8831613.3067",
    "estimated_apy_pct": "5.89",
    "fee_note": "Epoch fee: ~5% of staking rewards. Deposit fee: 0%. Withdrawal fee: ~0.3% (delayed unstake).",
    "unstake_note": "Unstaking creates a stake account that unlocks after the current epoch (~2-3 days)."
  }
}
```

---

### positions — Query User Positions

Query the user's JitoSOL balance and SOL equivalent value.

**Trigger phrases:**
- "Show my Jito staking position"
- "How much JitoSOL do I have?"
- "Check my JitoSOL balance"
- "Jito staking portfolio"

**Usage:**
```bash
jito positions --chain 501
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "wallet": "DTEqFXyFM9aMSGu9sw3PpRsZce6xqqmaUbGkFjmeieGE",
    "jitosol_ata": "9XyZ...",
    "jitosol_balance": "0.008000000",
    "jitosol_raw": "8000000",
    "sol_value": "0.010170209",
    "sol_per_jitosol": "1.27127624",
    "chain": "Solana"
  }
}
```

---

### stake — Stake SOL to Receive JitoSOL

Stake SOL into the Jito liquid staking pool to receive JitoSOL tokens.

**Trigger phrases:**
- "Stake 0.001 SOL on Jito"
- "Deposit SOL into JitoSOL"
- "Stake SOL with Jito"
- "Buy JitoSOL with 0.001 SOL"
- "Jito stake 0.001"

**Usage:**
```bash
# Preview (dry-run)
jito stake --amount 0.001 --chain 501 --dry-run

# Execute (after user confirms)
jito stake --amount 0.001 --chain 501
```

**Dry-run output:**
```json
{
  "ok": true,
  "dry_run": true,
  "data": {
    "operation": "stake",
    "sol_amount": 0.001,
    "lamports": "1000000",
    "expected_jitosol": "0.000786611",
    "sol_per_jitosol_rate": "1.27127624",
    "note": "Ask user to confirm before submitting the stake transaction",
    "unstake_note": "JitoSOL earns MEV-enhanced staking rewards (~5-10% APY)"
  }
}
```

**Agent flow:**
1. Run `--dry-run` to preview the stake operation
2. Show user the expected JitoSOL amount and current rate
3. **Ask user to confirm** before proceeding with the actual transaction
4. Execute `jito stake --amount <amount> --chain 501` via `onchainos wallet contract-call`
5. Return txHash with solscan.io link

**Important:** Always ask user to confirm before executing write operations. This command calls `onchainos wallet contract-call` to broadcast the transaction.

---

### unstake — Unstake JitoSOL Back to SOL

Initiate unstaking of JitoSOL. Creates a stake account that unlocks after the current epoch (~2-3 days).

**Trigger phrases:**
- "Unstake 0.005 JitoSOL on Jito"
- "Redeem JitoSOL for SOL"
- "Unstake from Jito"
- "Withdraw JitoSOL"

**Usage:**
```bash
# Preview (dry-run)
jito unstake --amount 0.005 --chain 501 --dry-run
```

**Dry-run output:**
```json
{
  "ok": true,
  "dry_run": true,
  "data": {
    "operation": "unstake",
    "jitosol_amount": 0.005,
    "expected_sol": "0.006356381",
    "delay_note": "Unstaking creates a stake account that unlocks after the current epoch (~2-3 days). You will need to manually deactivate and withdraw the stake account after the epoch ends.",
    "fee_note": "Unstake fee: ~0.3% of withdrawn amount",
    "note": "Ask user to confirm before submitting the unstake transaction"
  }
}
```

**Important:** Unstaking from Jito involves a delayed process:
1. JitoSOL is burned and a stake account is created
2. The stake account must wait until the epoch ends (~2-3 days)
3. After epoch ends, the stake account can be deactivated and withdrawn

Always ask user to confirm before executing write operations. This command calls `onchainos wallet contract-call` to broadcast the transaction.

---

## Notes

- JitoSOL is the liquid staking token — it automatically accrues SOL staking + MEV rewards
- The exchange rate increases over time as rewards accrue (1 JitoSOL = ~1.27 SOL currently)
- Minimum stake: 0.0001 SOL (100,000 lamports)
- Deposit fee: 0% | Withdrawal fee: ~0.3% | Epoch fee: ~5% of rewards
- All write operations are submitted via `onchainos wallet contract-call --chain 501`

## Protocol Addresses

| Name | Address |
|------|---------|
| Jito Stake Pool | `Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb` |
| JitoSOL Mint | `J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn` |
| SPL Stake Pool Program | `SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy` |
