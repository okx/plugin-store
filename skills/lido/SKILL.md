---
name: lido
description: Stake ETH with Lido liquid staking protocol to receive stETH, manage withdrawals, and track staking rewards. Supports staking, balance queries, withdrawal requests, withdrawal status, and claiming finalized withdrawals on Ethereum mainnet.
---

# Lido Liquid Staking Plugin

## Overview

This plugin enables interaction with the Lido liquid staking protocol on Ethereum mainnet (chain ID 1). Users can stake ETH to receive stETH (a rebasing liquid staking token), request withdrawals back to ETH, and claim finalized withdrawals.

**Key facts:**
- stETH is a rebasing token: balance grows daily without transfers
- Staking and withdrawals are only supported on Ethereum mainnet
- Withdrawal finalization typically takes 1–5 days (longer during Bunker mode)
- All write operations require user confirmation before submission

## Architecture

- Read ops (balance, APR, withdrawal status) → direct eth_call via onchainos or Lido REST API
- Write ops → after user confirmation, submits via `onchainos wallet contract-call`

## Pre-flight Checks

Before running any command:
1. Verify `onchainos` is installed: `onchainos --version` (requires ≥ 2.0.0)
2. For write operations, verify wallet is logged in: `onchainos wallet balance --chain 1 --output json`
3. If wallet check fails, prompt: "Please log in with `onchainos wallet login` first."

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|---|---|
| stETH (Lido) | `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84` |
| wstETH | `0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0` |
| WithdrawalQueueERC721 | `0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1` |

---

## Commands

### `stake` — Stake ETH

Deposit ETH into the Lido protocol to receive stETH.

**Usage:**
```
lido stake --amount-eth <ETH_AMOUNT> [--referral <ADDR>] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount-eth` | Yes | ETH amount to stake (e.g. `1.5`) |
| `--referral` | No | Referral address (defaults to zero address) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Check `isStakingPaused()` on stETH contract — abort if true
2. Call `get-apy` to fetch current APR for display
3. Show user: staking amount, current APR, expected stETH output, and contract address
4. **Ask user to confirm** the transaction before submitting
5. Execute: `onchainos wallet contract-call --chain 1 --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 --amt <WEI> --input-data 0xa1903eab<REFERRAL_PADDED>`

**Example:**
```bash
# Stake 1 ETH with no referral
lido stake --amount-eth 1.0

# Dry run to preview calldata
lido stake --amount-eth 2.5 --dry-run
```

**Calldata structure:** `0xa1903eab` + 32-byte zero-padded referral address

---

### `get-apy` — Get Current stETH APR

Fetch the 7-day simple moving average APR for stETH staking. No wallet required.

**Usage:**
```
lido get-apy
```

**Steps:**
1. HTTP GET `https://eth-api.lido.fi/v1/protocol/steth/apr/sma`
2. Display: "Current 7-day average stETH APR: X.XX%"

**Example output:**
```
Current 7-day average stETH APR: 3.20%
Note: This is post-10%-fee rate. Rewards are paid daily and compound automatically.
```

**No onchainos command required** — pure REST API call.

---

### `balance` — Check stETH Balance

Query stETH balance for an address.

**Usage:**
```
lido balance [--address <ADDR>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--address` | No | Address to query (resolved from onchainos if omitted) |

**Steps:**
1. Call `balanceOf(address)` on stETH contract
2. Call `sharesOf(address)` for precise share count
3. Display balance in ETH and wei

**Calldata:**
```
# balanceOf
onchainos wallet contract-call --chain 1 --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 \
  --input-data 0x70a08231000000000000000000000000<ADDRESS_32_BYTES> --read-only

# sharesOf
onchainos wallet contract-call --chain 1 --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 \
  --input-data 0xf5eb42dc000000000000000000000000<ADDRESS_32_BYTES> --read-only
```

**Note:** stETH is a rebasing token — balance grows daily without transfers. Always fetch fresh from chain.

---

### `request-withdrawal` — Request stETH Withdrawal

Lock stETH in the withdrawal queue and receive an unstETH NFT representing the withdrawal right.

**Usage:**
```
lido request-withdrawal --amount-eth <ETH_AMOUNT> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount-eth` | Yes | stETH amount to withdraw (e.g. `1.0`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**This operation requires two transactions:**

**Transaction 1 — Approve stETH:**
1. Show user: amount to approve, spender (WithdrawalQueueERC721), from address
2. **Ask user to confirm** the approve transaction before submitting
3. Execute: `onchainos wallet contract-call --chain 1 --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 --input-data 0x095ea7b3<WITHDRAWAL_QUEUE_PADDED><AMOUNT_PADDED>`

**Transaction 2 — Request Withdrawal:**
1. Show user: stETH amount, owner address, expected NFT (unstETH)
2. **Ask user to confirm** the withdrawal request transaction before submitting
3. Execute: `onchainos wallet contract-call --chain 1 --to 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1 --input-data <ABI_ENCODED_requestWithdrawals>`

**Constraints:**
- Minimum: 100 wei
- Maximum: 1,000 ETH (1e21 wei) per request
- Rewards stop accruing once stETH is locked in the queue

**Expected wait:** 1–5 days under normal conditions. Display wait time estimate from `https://wq-api.lido.fi/v2/request-time/calculate?amount=<WEI>`.

---

### `get-withdrawals` — List Withdrawal Requests

Query all pending and past withdrawal requests for an address.

**Usage:**
```
lido get-withdrawals [--address <ADDR>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--address` | No | Address to query (resolved from onchainos if omitted) |

**Steps:**
1. Call `getWithdrawalRequests(address)` → returns `uint256[]` of request IDs
   ```
   onchainos wallet contract-call --chain 1 --to 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1 \
     --input-data 0x7d031b65000000000000000000000000<ADDRESS> --read-only
   ```
2. Call `getWithdrawalStatus(uint256[])` → returns array of `WithdrawalRequestStatus` structs
3. Fetch estimated wait times from `https://wq-api.lido.fi/v2/request-time?ids=<ID>`
4. Display each request: ID, amount, status (PENDING / READY TO CLAIM / CLAIMED), estimated wait

**Status fields per request:**
- `amountOfStETH` — stETH locked at request time
- `isFinalized` — true when ETH is claimable
- `isClaimed` — true after ETH has been claimed

---

### `claim-withdrawal` — Claim Finalized Withdrawal

Claim ETH for finalized withdrawal requests. Burns the unstETH NFT and sends ETH to wallet.

**Usage:**
```
lido claim-withdrawal --ids <ID1,ID2,...> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--ids` | Yes | Comma-separated request IDs (e.g. `12345,67890`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**

**Step 1 — Get last checkpoint index (read-only):**
```
onchainos wallet contract-call --chain 1 --to 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1 \
  --input-data 0x526eae3e --read-only
```

**Step 2 — Find checkpoint hints (read-only):**
```
onchainos wallet contract-call --chain 1 --to 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1 \
  --input-data <ABI_ENCODED: 0x62abe3fa + requestIds[] + firstIndex(1) + lastCheckpointIndex> --read-only
```

**Step 3 — Claim:**
1. Show user: request IDs, hints, ETH expected, recipient address
2. **Ask user to confirm** the claim transaction before submitting
3. Execute: `onchainos wallet contract-call --chain 1 --to 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1 --input-data <ABI_ENCODED: 0xe3afe0a3 + requestIds[] + hints[]>`

**Pre-requisite:** All requests must have `isFinalized == true`. Check with `lido get-withdrawals` first.

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "Lido staking is currently paused" | DAO paused staking | Try again later; check Lido status page |
| "Cannot get wallet address" | Not logged in to onchainos | Run `onchainos wallet login` |
| "Amount below minimum 100 wei" | Withdrawal amount too small | Increase withdrawal amount |
| "Amount exceeds maximum" | Withdrawal > 1000 ETH | Split into multiple requests |
| "Hint count does not match" | Some requests not yet finalized | Check status with `get-withdrawals` first |
| HTTP 429 from Lido API | Rate limited | Wait and retry with exponential backoff |

## Suggested Follow-ups

After **stake**: suggest checking balance with `lido balance`, or viewing APR with `lido get-apy`.

After **request-withdrawal**: suggest monitoring status with `lido get-withdrawals`.

After **get-withdrawals**: if any request shows "READY TO CLAIM", suggest `lido claim-withdrawal --ids <ID>`.

After **claim-withdrawal**: suggest checking ETH balance via `onchainos wallet balance --chain 1`.

## Skill Routing

- For SOL liquid staking → use the `jito` skill
- For wallet balance queries → use `onchainos wallet balance`
- For general DeFi operations → use the appropriate protocol plugin
