---
name: kelp
description: "Kelp DAO rsETH liquid restaking plugin. Stake ETH or LSTs (stETH, ETHx, sfrxETH) to receive rsETH, a Liquid Restaking Token earning EigenLayer restaking rewards and staking APY. Supports apy, rates, positions, stake, unstake on Ethereum mainnet (chain 1) and rsETH bridged chains (Base, Arbitrum)."
---

# Kelp DAO rsETH Liquid Restaking Plugin

## Overview

Kelp DAO is a liquid restaking protocol built on EigenLayer. Users deposit ETH or LSTs to receive **rsETH** — a Liquid Restaking Token that accrues:
- EigenLayer restaking rewards
- Underlying LST staking yields (stETH, ETHx, sfrxETH)
- Kelp DAO protocol incentives

**Key facts:**
- rsETH is NOT a rebasing token — it appreciates in ETH value over time
- Primary chain: Ethereum Mainnet (chain ID 1)
- rsETH can be bridged to Base, Arbitrum, and Optimism
- Withdrawals go through a queue (several days wait time)
- All write operations require user confirmation before submission

## Architecture

- Read ops (apy, rates, positions) → direct eth_call via publicnode RPC + CoinGecko API
- Write ops (stake, unstake) → after user confirmation, submits via `onchainos wallet contract-call --force`

## Pre-flight Checks

Before running any command:
1. Verify `onchainos` is installed and wallet is logged in
2. For write operations: `onchainos wallet balance --chain 1 --output json`
3. If wallet check fails, prompt: "Please log in with `onchainos wallet login` first."

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|---|---|
| rsETH Token | `0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7` |
| LRTDepositPool | `0x036676389e48133B63a802f8635AD39E752D375D` |
| LRTOracle | `0x349A73444b1a310BAe67ef67973022020d70020d` |
| LRTWithdrawalManager | `0x62De59c08eB5dAE4b7E6F7a8cAd3006d6965ec16` |

---

## Commands

### `apy` — Get Current rsETH APY

Fetch current estimated APY for rsETH liquid restaking. No wallet required.

**Usage:**
```
kelp apy
```

**Data Sources:**
1. CoinGecko API for rsETH price and price change data
2. Annualizes 7-day ETH price change to estimate yield
3. Typical range: 4-7% combining EigenLayer restaking + staking rewards

**Example output:**
```
=== Kelp DAO rsETH APY ===
rsETH Price:       1.076000 ETH  ($2189.13 USD)
Estimated APY:     ~4.8% (annualized from 7d ETH price change)

Yield Sources:
  • EigenLayer restaking rewards
  • Underlying LST staking rewards (stETH, ETHx, sfrxETH)
  • Kelp DAO points (KELP token allocation)
```

---

### `rates` — Get Exchange Rates

Get current rsETH/ETH exchange rate from LRTOracle on-chain.

**Usage:**
```
kelp rates [--chain <CHAIN_ID>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--chain` | No | Chain ID (default: 1) |

**Data sources:**
1. `LRTOracle.rsETHPrice()` — on-chain oracle price
2. `LRTDepositPool.getRsETHAmountToMint(ETH_ADDR, 1e18)` — actual deposit rate
3. CoinGecko for USD reference price

**Example output:**
```
=== Kelp DAO rsETH Exchange Rates ===
rsETH/ETH Price:   1.07600000 ETH per rsETH
rsETH/USD Price:   $2189.13 USD
Deposit Rate:      1 ETH → 0.92940000 rsETH
```

---

### `positions` — Check rsETH Holdings

Query rsETH balance and underlying ETH value for an address.

**Usage:**
```
kelp positions [--address <ADDR>] [--chain <CHAIN_ID>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--address` | No | Address to query (resolved from onchainos if omitted) |
| `--chain` | No | Chain ID (default: 1) |

**Steps:**
1. Call `rsETH.balanceOf(address)` → raw rsETH balance
2. Call `LRTOracle.rsETHPrice()` → current ETH rate
3. Compute ETH value = balance × rate
4. Fetch USD price from CoinGecko

**Example output:**
```
=== Kelp DAO rsETH Positions ===
Address: 0x87fb...
rsETH Balance:     0.04640000 rsETH (46400000000000000 wei)
rsETH/ETH Rate:    1.07600000 ETH per rsETH
ETH Value:         0.04992640 ETH
USD Value:         $101.75
```

---

### `stake` — Stake ETH → rsETH

Deposit ETH into Kelp DAO via LRTDepositPool and receive rsETH.

**Usage:**
```
kelp stake --amount <ETH> [--chain <CHAIN_ID>] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | ETH amount to deposit (e.g. `0.1`) |
| `--chain` | No | Chain ID (default: 1) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Preview calldata without broadcasting |

**Steps:**
1. Resolve wallet address via `onchainos wallet addresses`
2. Fetch current rsETH rate from LRTOracle
3. Compute expected rsETH output via `getRsETHAmountToMint`
4. Build calldata: `depositETH(0, "")` — selector `0x72c51c0b`
5. Display: amount, expected rsETH, rate, contract
6. **Ask user to confirm** the transaction before submitting
7. Execute: `onchainos wallet contract-call --chain 1 --to <DEPOSIT_POOL> --amt <WEI> --input-data <CALLDATA> --force`

**Calldata structure:**
```
0x72c51c0b
0000...0000  (minRSETHAmountExpected = 0)
0000...0040  (offset to string = 64 bytes)
0000...0000  (string length = 0, empty referralId)
```

**Note:** Minimum deposit threshold may apply. If deposit is rejected, the contract will revert. Verify minimum requirements at kelpdao.xyz before staking.

**Example:**
```bash
# Dry run to preview calldata
kelp stake --amount 0.1 --dry-run

# Actual stake
kelp stake --amount 0.5 --chain 1
```

---

### `unstake` — Initiate rsETH Withdrawal

Initiate withdrawal of rsETH back to ETH via LRTWithdrawalManager.

**Usage:**
```
kelp unstake --amount <RSETH> [--chain <CHAIN_ID>] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | rsETH amount to withdraw (e.g. `0.05`) |
| `--chain` | No | Chain ID (default: 1) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Preview calldata without broadcasting |

**Steps:**
1. Resolve wallet address
2. Fetch rsETH/ETH rate from oracle
3. Compute expected ETH payout
4. Build calldata: `initiateWithdrawal(ETH_ADDR, rsEthAmountWei)` — selector `0xc8393ba9`
5. Display: amount, expected ETH, wait time warning
6. **Ask user to confirm** the transaction before submitting
7. Execute: `onchainos wallet contract-call --chain 1 --to <WITHDRAWAL_MANAGER> --input-data <CALLDATA> --force`

**Important:**
- Withdrawals join a queue with a wait period (typically several days)
- After queue finalization, call `completeWithdrawal` to claim ETH
- You will NOT receive ETH immediately after this transaction

**Example:**
```bash
# Dry run
kelp unstake --amount 0.05 --dry-run

# Actual unstake
kelp unstake --amount 0.05 --chain 1
```

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "Cannot get wallet address" | Not logged in to onchainos | Run `onchainos wallet login` |
| "Stake amount must be greater than 0" | Invalid amount | Provide a positive ETH amount |
| "Transaction failed" | Contract revert (e.g. below minimum deposit) | Check minimum deposit requirements |
| "eth_call RPC error" | RPC node issue | Retry; check network connectivity |
| HTTP 429 from CoinGecko | Rate limited | Wait and retry |

## Suggested Follow-ups

After **stake**: check balance with `kelp positions --chain 1`; view current rate with `kelp rates`.

After **unstake**: monitor withdrawal queue status on kelpdao.xyz or kerneldao.com/kelp.

After **apy**: if yield is satisfactory, proceed with `kelp stake`.

After **positions**: if you want to exit, use `kelp unstake --amount <BALANCE>`.

## Skill Routing

- For ETH-only staking without restaking → use the `lido` skill (stETH)
- For SOL liquid staking → use the `jito` skill
- For Aave/lending with rsETH collateral → use `aave-v3` skill
- For wallet balance queries → use `onchainos wallet balance`
- For EigenLayer direct restaking → kelp wraps this automatically
