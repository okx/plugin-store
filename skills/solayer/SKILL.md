---
name: solayer
description: "Solayer liquid restaking on Solana. Stake SOL to receive sSOL and earn restaking rewards. Trigger phrases: stake SOL Solayer, get sSOL, Solayer staking, Solayer rates, check sSOL balance, Solayer positions, restake SOL, sSOL APY"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

## Architecture

- **Read ops** (`rates`, `positions`) → direct REST API / Solana RPC; no confirmation needed
- **Write ops** (`stake`) → after user confirmation, submits serialized transaction via `onchainos wallet contract-call --chain 501 --unsigned-tx <base58_tx> --force`
- **`unstake`** → REST API not available; returns guidance to use Solayer UI

## Commands

### rates — Get sSOL staking rates

**Trigger:** "show Solayer rates", "what's the sSOL APY", "Solayer staking yield"

```
solayer rates [--chain 501]
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "apy_percent": 6.69,
    "ssol_to_sol": 1.14403543,
    "sol_to_ssol": 0.87408,
    "tvl_sol": "698250.11",
    "tvl_usd": "20643587.56",
    "epoch": 951,
    "epoch_remaining": "11h7m52s",
    "ssol_holders": 244951
  }
}
```

---

### positions — Check sSOL balance

**Trigger:** "show my Solayer positions", "how much sSOL do I have", "check sSOL balance"

```
solayer positions [--chain 501]
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "wallet": "DTEq...",
    "ssol_balance": 0.001234,
    "sol_value": 0.001412,
    "ssol_to_sol_rate": 1.14403,
    "apy_percent": 6.69
  }
}
```

---

### stake — Stake SOL to receive sSOL

**Trigger:** "stake SOL on Solayer", "restake SOL for sSOL", "put 0.001 SOL into Solayer"

1. Run `--dry-run` to preview the transaction
2. **Ask user to confirm** before proceeding with the on-chain transaction
3. Execute: `solayer stake --amount <amount>` → routes SOL → sSOL via `onchainos swap execute` (Jupiter DEX routing)

```
solayer stake --amount <sol_amount> [--chain 501] [--dry-run]
```

**Parameters:**
- `--amount` (required): SOL amount in UI units (e.g. `0.001`)

**Output:**
```json
{
  "ok": true,
  "data": {
    "txHash": "5Kx...",
    "amount_sol": 0.001,
    "ssol_received": 0.000873,
    "ssol_mint": "sSo14endRuUbvQaJS3dq36Q829a3A6BEfoeeRGJywEh",
    "description": "Staked 0.001 SOL → 0.000873 sSOL"
  }
}
```

---

### unstake — Unstake sSOL to receive SOL

**Trigger:** "unstake sSOL from Solayer", "redeem sSOL", "withdraw from Solayer"

1. Run `--dry-run` to see information
2. **Ask user to confirm** before directing them to the UI
3. Returns guidance to use Solayer app (REST API not available for unstaking)

```
solayer unstake --amount <ssol_amount> [--chain 501] [--dry-run]
```

**Parameters:**
- `--amount` (required): sSOL amount to unstake

**Note:** Unstaking requires complex multi-step on-chain instructions not available via REST API. Users must use the Solayer UI at https://app.solayer.org

---

## Key Contract Addresses

| Name | Address |
|------|---------|
| Restaking Program | `sSo1iU21jBrU9VaJ8PJib1MtorefUV4fzC9GURa2KNn` |
| sSOL Mint | `sSo14endRuUbvQaJS3dq36Q829a3A6BEfoeeRGJywEh` |
| Stake Pool | `po1osKDWYF9oiVEGmzKA4eTs8eMveFRMox3bUKazGN2` |

## Error Handling

- Invalid amount → clear error message
- API unavailable → retry with error description
- Insufficient SOL balance → error before submitting transaction
- Unstake not available via API → informational message with UI URL
