---
name: yearn-finance
description: "Yearn Finance yVaults — deposit, withdraw, and track auto-compounding yield on Ethereum. Trigger phrases: yearn, yvault, yearn deposit, yearn withdraw, yearn positions, yearn rates, yearn finance, yVault. Chinese: Yearn质押, Yearn存款, Yearn提款, 查看Yearn收益, Yearn金库"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Overview

Yearn Finance yVaults (v3) are ERC-4626 auto-compounding yield aggregators on Ethereum. Users deposit ERC-20 tokens (USDT, USDC, DAI, WETH) and receive vault shares that continuously accrue optimized yield via multiple DeFi strategies (Aave, Morpho, Compound, etc.).

This skill supports:
- **vaults** — list all active Yearn vaults with APR and TVL
- **rates** — show detailed APR history for vaults
- **positions** — query your vault share balances and underlying value
- **deposit** — deposit tokens into a vault (ERC-20 approve + ERC-4626 deposit)
- **withdraw** — redeem shares from a vault (ERC-4626 redeem)

## Architecture

- Read ops (vaults, rates, positions) → yDaemon REST API (`https://ydaemon.yearn.fi`) + direct `eth_call` via public RPC; no confirmation needed
- Write ops (deposit, withdraw) → after user confirmation, submits via `onchainos wallet contract-call`
- EVM chain: Ethereum mainnet (chain ID 1)
- Deposit flow: ERC-20 `approve()` → 3s delay → ERC-4626 `deposit()`

## Pre-flight Checks

- Binary installed: `which yearn-finance`
- onchainos logged in: `onchainos wallet addresses`
- Sufficient balance: `onchainos wallet balance --chain 1 --output json`

## Commands

### vaults — List Active Yearn Vaults

**Triggers:** "show yearn vaults", "list yvaults", "what yearn vaults are available", "yearn USDT vault"

```bash
yearn-finance vaults [--chain 1] [--token USDT]
```

**Parameters:**
- `--token` (optional): Filter by underlying token symbol (e.g. USDT, WETH, USDC)
- `--chain` (optional, default: 1): Chain ID

**Example output:**
```json
{
  "ok": true,
  "data": {
    "chain_id": 1,
    "count": 15,
    "vaults": [
      {
        "address": "0x310B7Ea7475A0B449Cfd73bE81522F1B88eFAFaa",
        "name": "USDT-1 yVault",
        "symbol": "yvUSDT-1",
        "token": { "symbol": "USDT", "decimals": 6 },
        "net_apr": "3.29%",
        "tvl_usd": "$7,604,530.73"
      }
    ]
  }
}
```

---

### rates — Show APR/APY Rates

**Triggers:** "yearn rates", "yearn APR", "what is yearn yield", "yearn USDT APR", "best yearn vault"

```bash
yearn-finance rates [--chain 1] [--token USDT]
```

**Parameters:**
- `--token` (optional): Filter by token symbol or vault name

**Example output:**
```json
{
  "ok": true,
  "data": {
    "rates": [
      {
        "name": "USDT-1 yVault",
        "token": "USDT",
        "net_apr": "3.29%",
        "history": { "week_ago": "3.40%", "month_ago": "3.10%" },
        "fees": { "performance": "10%", "management": "0%" }
      }
    ]
  }
}
```

---

### positions — Query Your Vault Holdings

**Triggers:** "my yearn positions", "yearn balance", "how much is in my yearn vault", "yearn holdings"

```bash
yearn-finance positions [--chain 1] [--wallet 0x...]
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
        "vault_name": "USDT-1 yVault",
        "token": "USDT",
        "shares": "9.270123",
        "underlying_balance": "9.998765",
        "net_apr": "3.29%"
      }
    ]
  }
}
```

---

### deposit — Deposit Tokens into a Yearn Vault

**Triggers:** "deposit into yearn", "put USDT in yearn vault", "yearn deposit 0.01 USDT", "invest in yearn"

```bash
yearn-finance deposit --vault <address_or_symbol> --amount <amount> [--chain 1] [--dry-run]
```

**Parameters:**
- `--vault` (required): Vault address (0x...) or token symbol (e.g. "USDT", "yvUSDT-1")
- `--amount` (required): Amount to deposit (e.g. "0.01")
- `--dry-run` (optional): Simulate without broadcasting

**Execution Flow:**
1. Fetch vault details from yDaemon API
2. Run `--dry-run` to preview calldata
3. **Ask user to confirm** before proceeding with on-chain transactions
4. Step 1: Submit ERC-20 `approve()` via `onchainos wallet contract-call` (selector `0x095ea7b3`)
5. Wait 3 seconds for approve confirmation
6. Step 2: Submit ERC-4626 `deposit()` via `onchainos wallet contract-call` (selector `0x6e553f65`)
7. Return deposit txHash and Etherscan link

**Example:**
```bash
yearn-finance --chain 1 deposit --vault USDT --amount 0.01
```

---

### withdraw — Redeem Shares from a Yearn Vault

**Triggers:** "withdraw from yearn", "redeem yearn shares", "exit yearn vault", "pull money from yearn"

```bash
yearn-finance withdraw --vault <address_or_symbol> [--shares <amount>] [--chain 1] [--dry-run]
```

**Parameters:**
- `--vault` (required): Vault address (0x...) or token symbol (e.g. "USDT", "yvUSDT-1")
- `--shares` (optional): Number of shares to redeem (omit to redeem all)
- `--dry-run` (optional): Simulate without broadcasting

**Execution Flow:**
1. Query user's current shares balance via `eth_call balanceOf()`
2. Run `--dry-run` to preview calldata
3. **Ask user to confirm** before submitting the withdrawal
4. Submit ERC-4626 `redeem()` via `onchainos wallet contract-call` (selector `0xba087652`)
5. Return txHash and Etherscan link

**Example:**
```bash
yearn-finance --chain 1 withdraw --vault USDT  # redeem all shares
yearn-finance --chain 1 withdraw --vault 0x310B7... --shares 5.0
```

---

## Error Handling

| Error | Meaning | Fix |
|-------|---------|-----|
| "Vault not found for query" | Symbol not matched | Run `vaults` command first to get exact address |
| "No shares held in vault" | Zero balance for `withdraw --all` | Check `positions` first |
| "Could not resolve wallet address" | onchainos not logged in | Run `onchainos wallet addresses` |
| "onchainos returned empty output" | CLI not installed or not in PATH | Check `which onchainos` |
| "yDaemon API error: 404" | Vault address invalid or wrong chain | Verify address and chain ID |

## Routing Rules

- For Yearn yield: always use this skill
- For Aave/Compound direct supply (not through Yearn): use their respective skills
- For token swaps to fund Yearn deposit: use the DEX skill first, then this skill
- Chain is always Ethereum (chain ID 1) for yVaults v3

## Notes

- yVaults v3 are ERC-4626 compliant; shares accrue value automatically (no claim needed)
- USDT has 6 decimals; DAI/WETH have 18 decimals — amounts are handled automatically
- The `deposit` command executes 2 transactions: approve first, then deposit
- `pricePerShare` grows over time as strategies earn yield
