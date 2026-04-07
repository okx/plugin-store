---
name: stakestone
description: Stake ETH with StakeStone liquid staking protocol to receive STONE yield-bearing tokens, manage withdrawal requests, and track your staking position on Ethereum mainnet.
---

# StakeStone Liquid Staking Plugin

## Overview

This plugin enables interaction with the StakeStone liquid staking protocol on Ethereum mainnet (chain ID 1). Users can stake ETH to receive STONE (an omnichain yield-bearing ETH token), request withdrawals back to ETH, and check their staking position.

**Key facts:**
- STONE is a non-rebasing yield-bearing token: value accrues via exchange rate appreciation
- Current exchange rate: ~1.063 ETH per STONE (increases over time as yield accrues)
- Staking and vault operations are only available on Ethereum mainnet
- Withdrawals are processed in settlement rounds (periodic batches)
- All write operations require user confirmation before submission

## Architecture

- Read ops (rate, position, vault info) - direct eth_call via JSON-RPC
- Write ops - after user confirmation, submitted via `onchainos wallet contract-call`

## Pre-flight Checks

Before running any command:
1. For write operations, verify wallet is logged in: `onchainos wallet balance --chain 1`
2. If wallet check fails, prompt: "Please log in with `onchainos wallet login` first."

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|---|---|
| StoneVault | `0xA62F9C5af106FeEE069F38dE51098D9d81B90572` |
| STONE token | `0x7122985656e38BDC0302Db86685bb972b145bD3C` |

---

## Commands

### `stake` - Stake ETH to receive STONE

Deposit ETH into the StakeStone vault to receive STONE liquid staking tokens.

**Usage:**
```
stakestone stake --amount <ETH_AMOUNT> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | ETH amount to stake (e.g. `0.001`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Fetch current exchange rate via `currentSharePrice()` to estimate STONE output
2. Show user: staking amount, estimated STONE output, current rate, contract address
3. **Ask user to confirm** the transaction before submitting
4. Execute: `onchainos wallet contract-call --chain 1 --to 0xA62F9C5af106FeEE069F38dE51098D9d81B90572 --amt <WEI> --input-data 0xd0e30db0 --force`

**Example:**
```bash
# Stake 0.001 ETH
stakestone stake --amount 0.001

# Dry run to preview
stakestone stake --amount 0.001 --dry-run
```

**Calldata structure:** `0xd0e30db0` (no parameters; ETH sent as `--amt`)

---

### `request-withdraw` - Request STONE withdrawal

Queue STONE tokens for withdrawal back to ETH. Processed in the next settlement round.

**Usage:**
```
stakestone request-withdraw --amount <STONE_AMOUNT> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | STONE amount to withdraw (e.g. `0.001`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Verify STONE balance is sufficient
2. Fetch current rate to estimate ETH return
3. Show user: STONE to withdraw, estimated ETH, withdrawal fee, expected round
4. **Ask user to confirm** the withdrawal request before submitting
5. Execute: `onchainos wallet contract-call --chain 1 --to 0xA62F9C5af106FeEE069F38dE51098D9d81B90572 --input-data 0x745400c9<SHARES_HEX_64> --force`

**Calldata structure:** `0x745400c9` + 32-byte uint256 (STONE shares in wei)

**Note:** Withdrawal is batched into settlement rounds. Monitor with `stakestone get-position`.

---

### `cancel-withdraw` - Cancel a pending withdrawal

Cancel STONE that was queued for withdrawal, returning it to your wallet.

**Usage:**
```
stakestone cancel-withdraw --amount <STONE_AMOUNT> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | STONE amount to cancel (e.g. `0.001`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Show user: STONE amount to cancel from queue, contract address
2. **Ask user to confirm** the cancel transaction before submitting
3. Execute: `onchainos wallet contract-call --chain 1 --to 0xA62F9C5af106FeEE069F38dE51098D9d81B90572 --input-data 0x9f01f7ba<SHARES_HEX_64> --force`

**Calldata structure:** `0x9f01f7ba` + 32-byte uint256 (STONE shares in wei)

---

### `get-rate` - Get current STONE exchange rate

Fetch live STONE/ETH exchange rate and vault statistics. No wallet required.

**Usage:**
```
stakestone get-rate
```

**Steps:**
1. eth_call `currentSharePrice()` - STONE price in ETH
2. eth_call `latestRoundID()` - current settlement round
3. eth_call `withdrawFeeRate()` - withdrawal fee percentage
4. eth_call `getVaultAvailableAmount()` - idle and deployed ETH

**Example output:**
```
STONE price:      1.063076 ETH per STONE
Settlement round: 274
Withdrawal fee:   0.0000%
Vault TVL:        10051.6504 ETH total
  Idle:           0.0050 ETH
  Deployed:       10051.6454 ETH
1 ETH stakes to approximately 0.940657 STONE
```

**No onchainos wallet command required** - pure eth_call reads.

---

### `get-position` - Get STONE position and pending withdrawal

Query STONE balance and pending withdrawal status for an address.

**Usage:**
```
stakestone get-position [--address <ADDR>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--address` | No | Address to query (resolved from onchainos if omitted) |

**Steps:**
1. eth_call `balanceOf(address)` on STONE token - current STONE balance
2. eth_call `currentSharePrice()` - convert to ETH value
3. eth_call `userReceipts(address)` on StoneVault - pending withdrawal info
4. Display full position summary

**Example output:**
```
STONE balance:    0.940657 STONE
ETH value:        0.999999 ETH (at 1.063076 ETH/STONE)

Pending Withdrawal: None
```

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "Cannot get wallet address" | Not logged in to onchainos | Run `onchainos wallet login` |
| "Insufficient STONE balance" | Trying to withdraw more than held | Check balance with `get-position` |
| "Stake amount must be greater than 0" | Invalid amount | Provide positive ETH amount |
| eth_call RPC error | RPC endpoint issue | Retry; publicnode may be temporarily unavailable |

## Suggested Follow-ups

After **stake**: suggest checking position with `stakestone get-position`.

After **request-withdraw**: suggest monitoring with `stakestone get-position`.

After **get-rate**: if rate looks favorable, suggest `stakestone stake --amount <X>`.

## Skill Routing

- For ETH liquid staking via Lido (stETH) - use the `lido` skill
- For wallet balance queries - use `onchainos wallet balance`
- For Solana liquid staking - use the `jito` skill
