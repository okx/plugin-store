---
name: rocket-pool
description: Interact with Rocket Pool, the decentralised Ethereum liquid staking protocol. Stake ETH to receive rETH (a non-rebasing liquid staking token that appreciates in value), check exchange rates, view protocol stats, manage positions, and burn rETH to redeem ETH. Supports Ethereum mainnet only.
---

# Rocket Pool Plugin

## Overview

Rocket Pool is a decentralised Ethereum liquid staking protocol. Users deposit ETH and receive **rETH** — a liquid staking token whose value increases relative to ETH as staking rewards accumulate.

**Key differences from Lido stETH:**
- rETH is non-rebasing: your balance stays constant, but each rETH is worth more ETH over time
- Fully decentralised: node operators run validators with their own bonded ETH + user deposits
- No lock-up: rETH can be traded on DEXes at any time
- Minimum deposit: 0.01 ETH (protocol-enforced)

**Chain support:** Ethereum Mainnet (chain ID: 1) only.

## Architecture

- Contract addresses resolved **dynamically** via RocketStorage (`0x1d8f8f00cfa6758d7bE78336684788Fb0ee0Fa46`)
- Read ops use direct JSON-RPC eth_call to `https://ethereum.publicnode.com`
- Write ops (stake, unstake) require explicit user confirmation before submitting to the network

## Pre-flight Checks

Before any command:
1. Verify `onchainos` is installed: `onchainos --version` (requires ≥ 2.2.0)
2. For write operations, verify wallet login: `onchainos wallet balance --chain 1 --output json`
3. If wallet check fails, prompt: "Please log in with `onchainos wallet login` first."

## Contract Addresses (Ethereum Mainnet, resolved dynamically)

| Contract | Resolved Address |
|---|---|
| RocketStorage | `0x1d8f8f00cfa6758d7bE78336684788Fb0ee0Fa46` |
| RocketDepositPool | `0xce15294273cfb9d9b628f4d61636623decdf4fdc` |
| RocketTokenRETH | `0xae78736cd615f374d3085123a210448e74fc6393` |
| RocketNetworkBalances | `0x1d9f14c6bfd8358b589964bad8665add248e9473` |
| RocketNodeManager | `0xcf2d76a7499d3acb5a22ce83c027651e8d76e250` |

---

## Commands

### `rate` — Get rETH Exchange Rate

Query the current ETH/rETH exchange rate from the rETH token contract.

**Usage:**
```
rocket-pool rate [--chain 1]
```

**Output:** Shows how many ETH 1 rETH is worth (e.g. 1.16 ETH/rETH).

**No wallet required.**

---

### `apy` — Get Staking APY

Fetch the current rETH staking APY from the Rocket Pool API, with on-chain exchange rate for context.

**Usage:**
```
rocket-pool apy [--chain 1]
```

**Output:** Current APY percentage, exchange rate.

**No wallet required.**

---

### `stats` — Protocol Statistics

Query Rocket Pool's TVL, node count, minipool count, rETH supply, and exchange rate.

**Usage:**
```
rocket-pool stats [--chain 1]
```

**Output:** All key protocol metrics from on-chain sources.

**No wallet required.**

---

### `positions` — Check rETH Balance

Query the rETH balance and ETH equivalent for a wallet address.

**Usage:**
```
rocket-pool positions [--chain 1] [--address <ADDR>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--address` | No | Address to query (resolved from onchainos wallet if omitted) |

**No wallet required for read.** Wallet login needed if `--address` is omitted.

---

### `stake` — Stake ETH for rETH

Deposit ETH into RocketDepositPool to receive rETH.

**Usage:**
```
rocket-pool stake [--chain 1] --amount <ETH> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | ETH to stake (minimum 0.01 ETH, e.g. `0.05`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Preview calldata without submitting |

**Steps:**
1. Validate amount ≥ 0.01 ETH (protocol minimum)
2. Resolve RocketDepositPool address from RocketStorage
3. Fetch current exchange rate to display expected rETH output
4. Show transaction details: amount, expected rETH, contract, calldata
5. **Ask user to confirm** the transaction before submitting
6. Execute: `onchainos wallet contract-call --chain 1 --to <RocketDepositPool> --amt <WEI> --input-data 0xd0e30db0 --force`

**Minimum deposit:** 0.01 ETH (enforced by the protocol contract)

**Example:**
```bash
rocket-pool stake --amount 0.05
rocket-pool stake --amount 0.1 --dry-run
```

---

### `unstake` — Burn rETH for ETH

Burn rETH tokens to receive ETH via `RocketTokenRETH.burn(uint256)`.

**Usage:**
```
rocket-pool unstake [--chain 1] --amount <rETH> [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | rETH amount to burn (e.g. `0.05`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Preview calldata without submitting |

**Steps:**
1. Validate rETH balance is sufficient
2. Check deposit pool ETH liquidity (warn if insufficient)
3. Fetch exchange rate to display expected ETH output
4. Show transaction details: rETH amount, expected ETH, contract, calldata
5. **Ask user to confirm** the transaction before submitting
6. Execute: `onchainos wallet contract-call --chain 1 --to <RocketTokenRETH> --input-data 0x42966c68<RETH_AMOUNT_32_BYTES> --force`

**Note:** If the deposit pool has insufficient ETH, the burn will fail. Consider trading rETH on a DEX (e.g. Uniswap, Curve) instead.

**Example:**
```bash
rocket-pool unstake --amount 0.05
rocket-pool unstake --amount 0.1 --dry-run
```

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "Minimum deposit is 0.01 ETH" | Amount below protocol minimum | Increase stake amount to at least 0.01 ETH |
| "Cannot resolve wallet address" | Not logged in | Run `onchainos wallet login` first |
| "Insufficient rETH balance" | Not enough rETH to burn | Check balance with `rocket-pool positions` |
| "Deposit pool may have insufficient ETH" | Pool empty | Trade rETH on a DEX instead |
| "RocketStorage returned zero address" | Contract upgrade | Plugin will auto-resolve new addresses on next run |

## Suggested Follow-ups

After **stake**: suggest checking balance with `rocket-pool positions`, or rate with `rocket-pool rate`.

After **unstake**: suggest checking ETH balance via `onchainos wallet balance --chain 1`.

After **positions** with non-zero balance: suggest `rocket-pool unstake` or `rocket-pool rate`.

After **stats**: suggest exploring node operation at https://rocketpool.net/node-operators.

## Skill Routing

- For SOL liquid staking → use the `jito` skill
- For ETH staking via Lido → use the `lido` skill
- For wallet balance queries → use `onchainos wallet balance`
- For rETH/ETH DEX swap → use uniswap/curve plugins
