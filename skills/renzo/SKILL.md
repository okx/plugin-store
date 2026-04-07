---
name: renzo
description: "Renzo EigenLayer liquid restaking protocol. Deposit ETH or stETH to receive ezETH and earn restaking rewards from EigenLayer AVS operators. Trigger phrases: deposit ETH into Renzo, restake ETH, get ezETH, Renzo APR, Renzo balance, Renzo TVL, liquid restake. Chinese: 存款到Renzo, 再质押ETH, 获取ezETH, Renzo年化收益率, Renzo余额"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Renzo EigenLayer Restaking Plugin

## Overview

This plugin enables interaction with the Renzo liquid restaking protocol on Ethereum mainnet (chain ID 1). Users can deposit native ETH or stETH to receive ezETH (a liquid restaking token representing EigenLayer restaking positions), check balances, view APR, and query protocol TVL.

**Key facts:**
- ezETH is a non-rebasing ERC-20; its exchange rate vs ETH appreciates over time
- Deposits are on Ethereum mainnet only (chain 1)
- No native withdrawal from Renzo currently; exit via DEX (swap ezETH → ETH)
- All write operations require user confirmation before submission

## Architecture

- Read ops (balance, APR, TVL) → direct eth_call via public RPC or Renzo REST API; no wallet needed
- Write ops (deposit-eth, deposit-steth) → after user confirmation, submits via `onchainos wallet contract-call`

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|---|---|
| RestakeManager (proxy) | `0x74a09653A083691711cF8215a6ab074BB4e99ef5` |
| ezETH token | `0xbf5495Efe5DB9ce00f80364C8B423567e58d2110` |
| stETH (Lido, accepted collateral) | `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84` |

---

## Commands

### `deposit-eth` — Deposit native ETH

Deposit ETH into Renzo RestakeManager to receive ezETH (liquid restaking token).

**Usage:**
```
renzo deposit-eth --amount-eth <ETH_AMOUNT> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount-eth` | Yes | ETH amount to deposit (e.g. `0.00005`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Preview calldata without broadcasting |

**Steps:**
1. Check `paused()` on RestakeManager — abort if true
2. Show user: deposit amount, contract address, expected ezETH output
3. **Ask user to confirm** the transaction before submitting
4. Execute: `onchainos wallet contract-call --chain 1 --to 0x74a09653A083691711cF8215a6ab074BB4e99ef5 --amt <WEI> --input-data 0xf6326fb3`

**Calldata structure:** `0xf6326fb3` (no parameters — ETH value in --amt)

**Example:**
```bash
# Deposit 0.00005 ETH (minimum test amount)
renzo deposit-eth --amount-eth 0.00005

# Dry run preview
renzo deposit-eth --amount-eth 0.1 --dry-run
```

---

### `deposit-steth` — Deposit stETH

Deposit stETH into Renzo to receive ezETH. Requires approve + deposit (two transactions).

**Usage:**
```
renzo deposit-steth --amount <STETH_AMOUNT> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | stETH amount to deposit (e.g. `0.00005`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Preview calldata without broadcasting |

**This operation may require two transactions:**

**Transaction 1 — Approve stETH (if allowance insufficient):**
1. Show user: amount to approve, spender (RestakeManager)
2. **Ask user to confirm** the approve transaction
3. Execute: `onchainos wallet contract-call --chain 1 --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 --input-data 0x095ea7b3<RESTAKE_MGR_PADDED><AMOUNT_PADDED>`

**Transaction 2 — Deposit stETH:**
1. Show user: stETH amount, deposit target
2. **Ask user to confirm** the deposit transaction
3. Execute: `onchainos wallet contract-call --chain 1 --to 0x74a09653A083691711cF8215a6ab074BB4e99ef5 --input-data 0x47e7ef24<STETH_PADDED><AMOUNT_PADDED>`

**Example:**
```bash
renzo deposit-steth --amount 0.00005
renzo deposit-steth --amount 0.1 --dry-run
```

---

### `get-apr` — Get current restaking APR

Fetch the current Renzo restaking APR from the Renzo API. No wallet required.

**Usage:**
```
renzo get-apr
```

**Steps:**
1. HTTP GET `https://api.renzoprotocol.com/apr`
2. Display: "Current Renzo APR: X.XX%"

**Example output:**
```json
{
  "ok": true,
  "data": {
    "apr_percent": 2.52,
    "apr_display": "2.5208%",
    "description": "Renzo ezETH restaking APR (annualized, EigenLayer + AVS rewards)"
  }
}
```

**No onchainos command required** — pure REST API call.

---

### `balance` — Check ezETH and stETH balances

Query ezETH and stETH balances for an address.

**Usage:**
```
renzo balance [--address <ADDR>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--address` | No | Address to query (resolved from onchainos if omitted) |

**Steps:**
1. Call `balanceOf(address)` on ezETH contract
2. Call `balanceOf(address)` on stETH contract
3. Display both balances

**No onchainos write command required** — read-only eth_call.

---

### `get-tvl` — Get protocol TVL

Query the total value locked in Renzo.

**Usage:**
```
renzo get-tvl
```

**Steps:**
1. Call `calculateTVLs()` on RestakeManager → extract totalTVL
2. Call `totalSupply()` on ezETH → display circulating supply
3. Display TVL in ETH

**No onchainos write command required** — read-only eth_call.

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "Renzo RestakeManager is currently paused" | Admin paused protocol | Try again later |
| "Cannot get wallet address" | Not logged in | Run `onchainos wallet login` |
| "Deposit amount must be greater than 0" | Zero amount | Provide valid amount |
| HTTP error from Renzo API | API unavailable | Retry |

## Suggested Follow-ups

After **deposit-eth** or **deposit-steth**: check balance with `renzo balance` or view APR with `renzo get-apr`.

After **balance**: if ezETH balance is 0, suggest `renzo deposit-eth --amount-eth 0.00005` to start earning restaking rewards.

## Skill Routing

- For ETH liquid staking (stETH) → use the `lido` skill
- For SOL liquid staking → use the `jito` skill
- For wallet balance queries → use `onchainos wallet balance`
