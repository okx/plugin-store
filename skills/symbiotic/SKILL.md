---
name: symbiotic
version: "0.1.0"
description: "Symbiotic restaking protocol — deposit collateral, manage positions, and check vault rates on Ethereum"
---

# Symbiotic Restaking Skill

Symbiotic is a permissionless shared security / restaking protocol on Ethereum. Users deposit collateral tokens (wstETH, rETH, cbETH, etc.) into Vaults to earn restaking rewards while securing decentralized networks.

## Overview

All write operations (deposit, withdraw) go through `onchainos wallet contract-call` after user confirmation. Read operations use the Symbiotic REST API and direct Ethereum RPC calls.

**Architecture:**
- Read ops → Symbiotic API (`app.symbiotic.fi/api/v2`) + Ethereum RPC eth_call
- Write ops → after **user confirmation**, submits via `onchainos wallet contract-call`

---

## Commands

### vaults — List Restaking Vaults

**Trigger phrases:**
- "Show Symbiotic vaults"
- "What vaults does Symbiotic have?"
- "List Symbiotic restaking options"
- "What tokens can I restake on Symbiotic?"

**Usage:**
```
symbiotic vaults [--token <SYMBOL>] [--limit <N>] [--chain <ID>]
```

**Parameters:**
- `--token` (optional): Filter by token symbol (e.g. `wstETH`, `rETH`, `cbETH`)
- `--limit` (optional, default 20): Max number of vaults to return
- `--chain` (optional, default 1): Ethereum chain ID

**Example output:**
```json
{
  "ok": true,
  "total": 20,
  "vaults": [
    {
      "address": "0xC329...",
      "name": "wstETH",
      "token_symbol": "wstETH",
      "tvl": "$59.3M",
      "apr": "2.37%"
    }
  ]
}
```

---

### positions — Check Your Restaking Positions

**Trigger phrases:**
- "Show my Symbiotic positions"
- "What's in my Symbiotic restaking portfolio?"
- "How much wstETH do I have restaked on Symbiotic?"
- "Check my Symbiotic balance"

**Usage:**
```
symbiotic positions [--address <ADDR>] [--chain <ID>]
```

**Parameters:**
- `--address` (optional): Wallet address (defaults to logged-in wallet)
- `--chain` (optional, default 1): Ethereum chain ID

---

### rates — Check Vault APR and Rewards

**Trigger phrases:**
- "What's the APR on Symbiotic vaults?"
- "Show me Symbiotic restaking rates"
- "Which Symbiotic vault has the best yield?"
- "What rewards does Symbiotic wstETH vault offer?"

**Usage:**
```
symbiotic rates [--token <SYMBOL>] [--limit <N>] [--chain <ID>]
```

---

### deposit — Deposit Collateral into a Vault

**Trigger phrases:**
- "Deposit 0.01 wstETH into Symbiotic"
- "Restake my wstETH on Symbiotic"
- "Put 0.01 rETH into Symbiotic vault"
- "Stake wstETH on Symbiotic"

**Usage:**
```
symbiotic deposit --token <SYMBOL> --amount <AMOUNT> [--vault <ADDR>] [--from <ADDR>] [--chain <ID>] [--dry-run]
```

**Parameters:**
- `--token` (optional, default wstETH): Collateral token symbol
- `--amount` (required): Amount in human-readable units (e.g. `0.01`)
- `--vault` (optional): Specific vault address (defaults to largest vault for the token)
- `--from` (optional): Wallet address (defaults to logged-in wallet)
- `--dry-run` (optional): Preview the transaction without broadcasting

**Implementation flow:**
1. Fetch vault info from Symbiotic API
2. Run `--dry-run` to preview the transaction details
3. **Ask user to confirm** the deposit amount and vault address before proceeding
4. Step 1: `onchainos wallet contract-call` to approve the collateral token
5. Step 2: `onchainos wallet contract-call` to call `deposit(address,uint256)` on the vault

**⚠️ Important:** Deposit involves two transactions — ERC-20 approve followed by vault deposit. Both require user confirmation.

---

### withdraw — Request Withdrawal from a Vault

**Trigger phrases:**
- "Withdraw 0.01 wstETH from Symbiotic"
- "Unstake my wstETH from Symbiotic"
- "Request withdrawal from Symbiotic vault"
- "Exit my Symbiotic position"

**Usage:**
```
symbiotic withdraw --token <SYMBOL> --amount <AMOUNT> [--vault <ADDR>] [--from <ADDR>] [--chain <ID>] [--dry-run]
```

**Parameters:**
- `--token` (optional, default wstETH): Collateral token symbol
- `--amount` (required): Amount to withdraw in human-readable units
- `--vault` (optional): Specific vault address
- `--from` (optional): Wallet address
- `--dry-run` (optional): Preview without broadcasting

**Implementation flow:**
1. Fetch vault info and check active balance
2. Query current epoch and epoch duration for timing info
3. Run `--dry-run` to preview the transaction
4. **Ask user to confirm** the withdrawal request and acknowledge the epoch waiting period
5. `onchainos wallet contract-call` to call `withdraw(address,uint256)` on the vault

**⚠️ Important:** Symbiotic withdrawals are epoch-based. After requesting withdrawal, funds are locked until the next epoch boundary (~7 days). A separate `claim` call is needed to receive the tokens after the epoch ends.

---

## Contract Information

| Contract | Address | Function |
|----------|---------|----------|
| wstETH Vault | `0xC329400492c6ff2438472D4651Ad17389fCb843a` | Symbiotic wstETH restaking vault |
| rETH Vault | `0x03Bf48b8a1B37FBeAd1EcAbcF15B98B924ffA5AC` | Symbiotic rETH restaking vault |
| cbETH Vault | `0xB26ff591F44b04E78de18f43B46f8b70C6676984` | Symbiotic cbETH restaking vault |
| wstETH Token | `0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0` | Lido wrapped stETH |

## Key Function Selectors

| Function | Selector | Verified |
|----------|---------|---------|
| `deposit(address,uint256)` | `0x47e7ef24` | cast sig |
| `withdraw(address,uint256)` | `0xf3fef3a3` | cast sig |
| `activeBalanceOf(address)` | `0x59f769a9` | cast sig |
| `collateral()` | `0xd8dfeb45` | cast sig |
| `currentEpoch()` | `0x76671808` | cast sig |
| `epochDuration()` | `0x4ff0876a` | cast sig |
| `approve(address,uint256)` | `0x095ea7b3` | cast sig |
