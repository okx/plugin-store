---
name: usde-staking
description: Stake USDe to earn yield via Ethena sUSDe ERC-4626 vault on Ethereum mainnet. Supports staking, unstaking with cooldown, claiming, rate queries, and position tracking.
---

# USDe Staking Plugin (Ethena sUSDe)

## Overview

This plugin enables interaction with Ethena's sUSDe staking vault on Ethereum mainnet (chain ID 1).
USDe is Ethena's synthetic dollar. Staking USDe mints sUSDe, an ERC-4626 vault token that
automatically earns yield from Ethena's protocol. sUSDe appreciates against USDe over time.

**Key facts:**
- sUSDe is an ERC-4626 vault — staking is a deposit, unstaking requires a cooldown period
- Cooldown period: ~1 day (86400 seconds, set by contract governance)
- Unstaking is a 2-step process: request-unstake (starts cooldown) then claim-unstake (after cooldown)
- All write operations require user confirmation before submission

## Architecture

- Read ops (rates, positions) - direct eth_call via Ethereum RPC + Ethena REST API
- Write ops - after user confirmation, submits via `onchainos wallet contract-call`

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|---|---|
| USDe token | `0x4c9EDD5852cd905f086C759E8383e09bff1E68B3` |
| sUSDe vault (ERC-4626) | `0x9D39A5DE30e57443BfF2A8307A4256c8797A3497` |

---

## Commands

### `get-rates` - Get sUSDe Staking Yield

Fetch current sUSDe staking APY and exchange rate from Ethena API and on-chain.

**Usage:**
```
usde-staking get-rates
```

**Returns:** Current APY, 30/90 day averages, USDe per sUSDe exchange rate, TVL, cooldown duration.

**Example:**
```bash
usde-staking get-rates
```

---

### `get-positions` - View sUSDe Staking Position

Query sUSDe balance, USDe equivalent value, and any pending unstake cooldown.

**Usage:**
```
usde-staking get-positions [--address <ADDR>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--address` | No | Wallet address to query (resolved from onchainos if omitted) |

**Example:**
```bash
# Query your own position
usde-staking get-positions

# Query a specific address
usde-staking get-positions --address 0x1234...
```

---

### `stake` - Stake USDe to Receive sUSDe

Deposit USDe into the sUSDe ERC-4626 vault to earn yield. Requires two transactions:
1. Approve USDe spend
2. Deposit USDe into sUSDe vault

**Usage:**
```
usde-staking stake --amount <USDE_AMOUNT> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | USDe amount to stake (e.g. `10.5`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Check USDe balance - abort if insufficient
2. Preview sUSDe output via `previewDeposit()`
3. Show: amount, expected sUSDe, current APY
4. **Ask user to confirm** before submitting transactions
5. Execute approve: `onchainos wallet contract-call --chain 1 --to 0x4c9EDD5852cd905f086C759E8383e09bff1E68B3 --input-data 0x095ea7b3...`
6. Wait 15 seconds for approve to confirm
7. Execute deposit: `onchainos wallet contract-call --chain 1 --to 0x9D39A5DE30e57443BfF2A8307A4256c8797A3497 --input-data 0x6e553f65...`

**Calldata structure:**
- Approve: `0x095ea7b3` + padded spender (sUSDe addr) + padded amount
- Deposit: `0x6e553f65` + padded assets (USDe wei) + padded receiver (wallet)

**Example:**
```bash
# Stake 10 USDe
usde-staking stake --amount 10.0

# Dry run preview
usde-staking stake --amount 100.0 --dry-run
```

---

### `request-unstake` - Initiate Unstake Cooldown

Start the cooldown period to unstake sUSDe. After the cooldown period, call `claim-unstake`.

**IMPORTANT:** This operation locks your sUSDe for the cooldown period (~1 day). Funds are not
at risk, but they cannot be moved until cooldown completes. This is a cooldown-gated operation.

**Usage:**
```
usde-staking request-unstake --shares <AMOUNT> [--from <ADDR>] [--dry-run]
usde-staking request-unstake --assets <AMOUNT> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--shares` | Yes* | sUSDe share amount to unstake (e.g. `10.5`) |
| `--assets` | Yes* | USDe asset amount to unstake (alternative to --shares) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

*One of `--shares` or `--assets` must be provided.

**Steps:**
1. Verify sUSDe balance is sufficient (if using --shares)
2. Show cooldown duration and amount
3. **Ask user to confirm** before initiating cooldown
4. Execute: `onchainos wallet contract-call --chain 1 --to 0x9D39A5DE30e57443BfF2A8307A4256c8797A3497 --input-data 0x9343d9e1...`

**Calldata structure:**
- By shares: `0x9343d9e1` + padded sUSDe amount (wei)
- By assets: `0xcdac52ed` + padded USDe amount (wei)

**Example:**
```bash
# Request unstake of 10 sUSDe
usde-staking request-unstake --shares 10.0

# Request unstake by USDe target amount
usde-staking request-unstake --assets 10.0 --dry-run
```

---

### `claim-unstake` - Claim USDe After Cooldown

Claim USDe after the cooldown period has completed. Must have previously called `request-unstake`.

**Usage:**
```
usde-staking claim-unstake [--receiver <ADDR>] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--receiver` | No | Address to receive USDe (defaults to wallet address) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Check cooldown status via `cooldowns(wallet)` - abort if not complete
2. Show claimable amount
3. **Ask user to confirm** before claiming
4. Execute: `onchainos wallet contract-call --chain 1 --to 0x9D39A5DE30e57443BfF2A8307A4256c8797A3497 --input-data 0xf2888dbb...`

**Calldata structure:** `0xf2888dbb` + padded receiver address

**Example:**
```bash
# Claim unstaked USDe
usde-staking claim-unstake

# Claim to a different address
usde-staking claim-unstake --receiver 0xabcd...

# Dry run
usde-staking claim-unstake --dry-run
```

---

## Unstaking Flow

```
1. usde-staking request-unstake --shares 10.0
   -> Initiates ~1 day cooldown
   -> sUSDe is locked, no longer earning yield

2. [Wait ~1 day for cooldown to complete]

3. usde-staking get-positions
   -> Check if cooldown has completed ("READY TO CLAIM")

4. usde-staking claim-unstake
   -> Receive USDe in your wallet
```

## Supported Chains

| Chain | Chain ID | Status |
|---|---|---|
| Ethereum mainnet | 1 | Supported |
