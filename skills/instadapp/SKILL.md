---
name: instadapp
description: "Instadapp Lite Vaults — deposit ETH, withdraw, and track yield on Ethereum. Supports iETH v1 (ETH vault) and iETHv2 (stETH ERC-4626 vault). Trigger phrases: instadapp, instadapp lite, iETH vault, iETHv2, instadapp deposit, instadapp withdraw, instadapp positions, instadapp rates, instadapp yield"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Overview

Instadapp Lite vaults are ETH yield-aggregation products on Ethereum. Users deposit ETH or stETH into the vaults and receive iETH/iETHv2 shares that accumulate yield from leveraged stETH/WETH positions across Aave, Compound, Spark, and Fluid.

**Two vaults:**
- **iETH (v1)** - `0xc383a3833A87009fD9597F8184979AF5eDFad019`: Accepts native ETH via `supplyEth()`. Yield from leveraged stETH/WETH. Current exchange price ~1.2 ETH per iETH.
- **iETHv2** - `0xa0d3707c569ff8c87fa923d3823ec5d81c98be78`: ERC-4626, accepts stETH. Aggregates across multiple protocols. Exchange price ~1.2 stETH per iETHv2.

This skill supports:
- **vaults** - list both Lite vaults with exchange price and TVL
- **rates** - show cumulative yield and exchange price details
- **positions** - query your iETH/iETHv2 share balances
- **deposit** - deposit ETH into iETH v1 vault (or stETH into iETHv2)
- **withdraw** - burn iETH/iETHv2 shares to receive ETH/stETH

## Architecture

- Read ops (vaults, rates, positions) - direct `eth_call` via `https://ethereum.publicnode.com`; no confirmation needed
- Write ops (deposit, withdraw) - after user confirmation, submits via `onchainos wallet contract-call`
- EVM chain: Ethereum mainnet (chain ID 1)
- iETH v1 deposit: single tx `supplyEth(address)` with ETH value (`--amt <wei>`)
- iETHv2 deposit: 2-tx flow: stETH `approve()` then ERC-4626 `deposit()`

## Pre-flight Checks

- Binary installed: `which instadapp`
- onchainos logged in: `onchainos wallet addresses`
- Sufficient ETH balance: `onchainos wallet balance --chain 1`

## Commands

### vaults - List Instadapp Lite Vaults

**Triggers:** "show instadapp vaults", "list instadapp lite vaults", "what instadapp vaults are available", "iETH vault info"

```bash
instadapp vaults [--chain 1]
```

**Example output:**
```json
{
  "ok": true,
  "data": {
    "chain_id": 1,
    "count": 2,
    "vaults": [
      {
        "address": "0xc383a3833A87009fD9597F8184979AF5eDFad019",
        "name": "Instadapp ETH",
        "symbol": "iETH",
        "version": "v1",
        "underlying": "ETH",
        "exchange_price_eth": "1.200478",
        "total_supply": "54.7737",
        "tvl_eth": "65.76"
      }
    ]
  }
}
```

---

### rates - Show Yield Rates

**Triggers:** "instadapp rates", "instadapp APY", "iETH yield", "instadapp lite yield", "instadapp exchange price"

```bash
instadapp rates [--chain 1]
```

**Example output:**
```json
{
  "ok": true,
  "data": {
    "rates": [
      {
        "symbol": "iETH",
        "exchange_price": "1.200478 ETH per iETH",
        "cumulative_yield_pct": "20.05%",
        "strategy": "Leveraged stETH/WETH yield via Aave V2/V3"
      }
    ]
  }
}
```

---

### positions - Query Your Holdings

**Triggers:** "my instadapp positions", "instadapp balance", "how much iETH do I have", "instadapp holdings", "check my iETH"

```bash
instadapp positions [--chain 1] [--wallet 0x...]
```

**Parameters:**
- `--wallet` (optional): Wallet address (default: resolved from onchainos)

**Example output:**
```json
{
  "ok": true,
  "data": {
    "wallet": "0x87fb...",
    "position_count": 1,
    "positions": [
      {
        "vault_name": "Instadapp ETH",
        "symbol": "iETH",
        "shares": "0.000041",
        "underlying_eth": "0.000049",
        "exchange_price": "1.200478"
      }
    ]
  }
}
```

---

### deposit - Deposit into Instadapp Lite Vault

**Triggers:** "deposit ETH into instadapp", "put ETH in instadapp lite", "instadapp deposit 0.0001 ETH", "buy iETH", "invest in instadapp lite"

```bash
instadapp deposit --vault v1 --amount <amount> [--chain 1] [--dry-run]
```

**Parameters:**
- `--vault` (optional, default: "v1"): "v1"/"iETH" for ETH vault, "v2"/"iETHv2" for stETH vault
- `--amount` (required): Amount to deposit (ETH for v1, stETH for v2, e.g. "0.0001")
- `--dry-run` (optional): Simulate without broadcasting

**Execution Flow (v1 - ETH vault):**
1. Parse amount, compute wei value
2. Run `--dry-run` to preview calldata
3. **Ask user to confirm** before proceeding with on-chain transaction
4. Submit `supplyEth(address)` via `onchainos wallet contract-call` with `--amt <wei>` (selector `0x87ee9312`)
5. Return deposit txHash and Etherscan link

**Execution Flow (v2 - stETH vault):**
1. Parse amount
2. Run `--dry-run` to preview calldata
3. **Ask user to confirm** before proceeding with on-chain transactions
4. Step 1: Submit stETH `approve()` via `onchainos wallet contract-call` (selector `0x095ea7b3`)
5. Wait 3 seconds for approve confirmation
6. Step 2: Submit ERC-4626 `deposit()` via `onchainos wallet contract-call` (selector `0x6e553f65`)
7. Return deposit txHash and Etherscan link

**Example:**
```bash
instadapp --chain 1 deposit --vault v1 --amount 0.0001
instadapp --chain 1 deposit --vault v1 --amount 0.0001 --dry-run
```

---

### withdraw - Withdraw from Instadapp Lite Vault

**Triggers:** "withdraw from instadapp", "redeem iETH", "exit instadapp lite vault", "sell iETH", "pull ETH from instadapp"

```bash
instadapp withdraw --vault v1 [--shares <amount>] [--chain 1] [--dry-run]
```

**Parameters:**
- `--vault` (optional, default: "v1"): "v1"/"iETH" or "v2"/"iETHv2"
- `--shares` (optional): Number of shares to redeem (omit to redeem all)
- `--dry-run` (optional): Simulate without broadcasting

**Execution Flow:**
1. Query user's current shares balance via `eth_call balanceOf()`
2. Run `--dry-run` to preview calldata
3. **Ask user to confirm** before submitting the withdrawal
4. iETH v1: Submit `withdraw(uint256,address)` via `onchainos wallet contract-call` (selector `0x00f714ce`)
5. iETHv2: Submit `redeem(uint256,address,address)` via `onchainos wallet contract-call` (selector `0xba087652`)
6. Return txHash and Etherscan link

**Example:**
```bash
instadapp --chain 1 withdraw --vault v1               # redeem all iETH shares
instadapp --chain 1 withdraw --vault v1 --shares 0.01 # redeem 0.01 iETH
```

---

## Error Handling

| Error | Meaning | Fix |
|-------|---------|-----|
| "No iETH shares held" | Zero balance for withdraw | Check `positions` first |
| "Could not resolve wallet address" | onchainos not logged in | Run `onchainos wallet addresses` |
| "onchainos returned empty output" | CLI not installed | Check `which onchainos` |
| "Invalid amount" | Non-numeric amount | Use format like "0.0001" |
| "Deposit failed" | Transaction reverted | Check ETH balance and vault status |

## Routing Rules

- For Instadapp Lite ETH yield: use this skill
- For ETH staking (Lido/Rocket Pool): use their respective skills
- For Aave/Compound direct lending: use their respective skills
- Chain is always Ethereum (chain ID 1) for Instadapp Lite vaults

## Important Notes

- Minimum test amount: 0.00005 ETH (GUARDRAILS limit)
- iETH v1 accepts native ETH directly — no ERC-20 approval needed
- iETHv2 requires stETH; use Lido skill first to convert ETH to stETH
- Exchange price grows over time as yield accumulates (1.0 at inception, currently ~1.2)
- Withdrawals may have a small fee (withdrawal fee set by protocol governance)
