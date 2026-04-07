---
name: kamino-liquidity
description: "Kamino Liquidity KVault earn vaults on Solana. Deposit tokens to earn yield, withdraw shares, and track positions. Trigger phrases: Kamino vault, Kamino liquidity, deposit to Kamino, Kamino earn, KVault, Kamino yield vault. Chinese: Kamino流动性, Kamino保险库, 存入Kamino, Kamino赚取收益"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Overview

Kamino Liquidity provides auto-compounding KVault earn vaults on Solana. Users deposit a single token (SOL, USDC, etc.) and receive shares representing their proportional stake. The vault automatically allocates liquidity to generate yield.

## Architecture

- **Read ops** (vaults, positions) → direct HTTP calls to `https://api.kamino.finance`; no confirmation needed
- **Write ops** (deposit, withdraw) → Kamino API builds the unsigned transaction → after user confirmation, submits via `onchainos wallet contract-call --chain 501 --unsigned-tx <base58_tx> --force`

## Execution Flow for Write Operations

1. Call Kamino API to build an unsigned serialized transaction
2. Run with `--dry-run` first to preview the transaction
3. **Ask user to confirm** before executing on-chain
4. Execute only after explicit user approval
5. Report transaction hash and link to solscan.io

---

## Commands

### vaults — List KVaults

Lists all available Kamino KVault earn vaults.

**Usage:**
```
kamino-liquidity vaults [--chain 501] [--token <filter>] [--limit <n>]
```

**Arguments:**
- `--chain` — Chain ID (must be 501, default: 501)
- `--token` — Filter by token symbol or name (optional, case-insensitive substring)
- `--limit` — Max vaults to show (default: 20)

**Trigger phrases:**
- "Show me Kamino vaults"
- "List Kamino liquidity vaults"
- "What Kamino KVaults are available?"
- "Show SOL vaults on Kamino"

**Example output:**
```json
{
  "ok": true,
  "chain": 501,
  "total": 115,
  "shown": 20,
  "vaults": [
    {
      "address": "GEodMsAREMV4JdKs1yUCTKpz4EtzxKoSDeM3NZkG1RRk",
      "name": "AL-SOL-aut-t",
      "token_mint": "So11111111111111111111111111111111111111112",
      "token_decimals": 9,
      "shares_mint": "...",
      "shares_issued": "122001000",
      "token_available": "221741",
      "performance_fee_bps": 0,
      "management_fee_bps": 0,
      "allocation_count": 2
    }
  ]
}
```

---

### positions — View user positions

Shows the user's current share balances across all Kamino KVaults.

**Usage:**
```
kamino-liquidity positions [--chain 501] [--wallet <address>]
```

**Arguments:**
- `--chain` — Chain ID (must be 501, default: 501)
- `--wallet` — Solana wallet address (optional; resolved from onchainos if omitted)

**Trigger phrases:**
- "Show my Kamino positions"
- "What Kamino vaults am I in?"
- "Check my Kamino liquidity holdings"

**Example output:**
```json
{
  "ok": true,
  "wallet": "DTEqFXyFM9aMSGu9sw3PpRsZce6xqqmaUbGkFjmeieGE",
  "chain": 501,
  "positions": [
    {
      "vault": "GEodMsAREMV4JdKs1yUCTKpz4EtzxKoSDeM3NZkG1RRk",
      "shares_amount": "0.001",
      "token_amount": "0.001001"
    }
  ]
}
```

---

### deposit — Deposit tokens into a KVault

Deposits tokens into a Kamino KVault and receives vault shares.

**Usage:**
```
kamino-liquidity deposit --vault <address> --amount <amount> [--chain 501] [--wallet <address>] [--dry-run]
```

**Arguments:**
- `--vault` — KVault address (base58, required)
- `--amount` — Amount to deposit in UI units (e.g. "0.001" for 0.001 SOL)
- `--chain` — Chain ID (must be 501, default: 501)
- `--wallet` — Solana wallet address (optional; resolved from onchainos if omitted)
- `--dry-run` — Preview transaction without broadcasting

**Trigger phrases:**
- "Deposit 0.001 SOL into Kamino vault GEodMs..."
- "Put 0.01 USDC into Kamino KVault"
- "Invest in Kamino liquidity vault"

**Important:** This operation submits a transaction on-chain.
- Run `--dry-run` first to preview
- **Ask user to confirm** before executing
- Execute: `onchainos wallet contract-call --chain 501 --to KvauGMspG5k6rtzrqqn7WNh3oZdyKqLKwK2XWQ8FLjd --unsigned-tx <base58_tx> --force`

**Example output:**
```json
{
  "ok": true,
  "vault": "GEodMsAREMV4JdKs1yUCTKpz4EtzxKoSDeM3NZkG1RRk",
  "wallet": "DTEqFXyFM9aMSGu9sw3PpRsZce6xqqmaUbGkFjmeieGE",
  "amount": "0.001",
  "data": {
    "txHash": "5xHk..."
  },
  "explorer": "https://solscan.io/tx/5xHk..."
}
```

---

### withdraw — Withdraw shares from a KVault

Redeems vault shares and receives back the underlying token.

**Usage:**
```
kamino-liquidity withdraw --vault <address> --amount <shares> [--chain 501] [--wallet <address>] [--dry-run]
```

**Arguments:**
- `--vault` — KVault address (base58, required)
- `--amount` — Number of shares to redeem (UI units, e.g. "1")
- `--chain` — Chain ID (must be 501, default: 501)
- `--wallet` — Solana wallet address (optional; resolved from onchainos if omitted)
- `--dry-run` — Preview transaction without broadcasting

**Trigger phrases:**
- "Withdraw from Kamino vault GEodMs..."
- "Redeem my Kamino shares"
- "Exit Kamino liquidity position"

**Important:** This operation submits a transaction on-chain.
- Run `--dry-run` first to preview
- **Ask user to confirm** before executing
- Execute: `onchainos wallet contract-call --chain 501 --to KvauGMspG5k6rtzrqqn7WNh3oZdyKqLKwK2XWQ8FLjd --unsigned-tx <base58_tx> --force`

**Example output:**
```json
{
  "ok": true,
  "vault": "GEodMsAREMV4JdKs1yUCTKpz4EtzxKoSDeM3NZkG1RRk",
  "wallet": "DTEqFXyFM9aMSGu9sw3PpRsZce6xqqmaUbGkFjmeieGE",
  "shares_redeemed": "0.5",
  "data": {
    "txHash": "7yBq..."
  },
  "explorer": "https://solscan.io/tx/7yBq..."
}
```

---

## Fund Limits (Testing)

- Max 0.001 SOL per deposit transaction
- SOL hard reserve: 0.002 SOL (never go below)
