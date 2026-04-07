---
name: moonwell
description: Moonwell Flagship lending/borrowing protocol (Compound V2 fork) — supply assets to earn interest, borrow against collateral, redeem mTokens, and claim WELL rewards. Supports Base, Optimism, and Moonbeam chains. Ask user to confirm before any write operation.
---

# Moonwell Flagship Plugin

## Overview

Moonwell is an open, non-custodial lending and borrowing protocol built on Base, Optimism, Moonbeam, and Moonriver. It is a fork of Compound V2 using **mTokens** (analogous to cTokens) with timestamp-based interest accrual.

**Key facts:**
- Supply assets → receive mTokens (representing your deposit + accrued interest)
- Borrow against collateral (requires overcollateralization)
- Earn WELL token rewards — claim via `claim-rewards`
- Borrow and repay are **dry-run only** for safety (liquidation risk)
- Primary chain: Base (8453)

## Pre-flight Checks

Before any command:
1. Verify `onchainos` is installed: `onchainos --version`
2. For write operations, check wallet balance: `onchainos wallet balance --chain 8453 --output json`
3. If wallet check fails: "Please log in with `onchainos wallet login` first."

## Contract Addresses (Base — Chain 8453)

| Contract | Address |
|---|---|
| Comptroller | `0xfBb21d0380beE3312B33c4353c8936a0F13EF26C` |
| WELL Token | `0xA88594D404727625A9437C3f886C7643872296AE` |
| mUSDC | `0xEdc817A28E8B93B03976FBd4a3dDBc9f7D176c22` |
| mWETH | `0x628ff693426583D9a7FB391E54366292F509D457` |
| mcbETH | `0x3bf93770f2d4a794c3d9EBEfBAeBAE2a8f09A5E5` |
| mUSDbC | `0x703843C3379b52F9FF486c9f5892218d2a065cC8` |
| mDAI | `0x73b06D8d18De422E269645eaCe15400DE7462417` |

---

## Commands

### `markets` — List Lending Markets

Query all Moonwell markets with real-time supply/borrow APR and exchange rates.

**Usage:**
```
moonwell markets [--chain 8453]
```

**Returns per market:** symbol, mToken address, supply APR%, borrow APR%, exchange rate (mToken → underlying).

**Example:**
```bash
moonwell markets --chain 8453
```

---

### `positions` — View Your Positions

Check your current supplied and borrowed balances across all Moonwell markets.

**Usage:**
```
moonwell positions [--chain 8453] [--wallet <ADDR>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--chain` | No | Chain ID (default: 8453) |
| `--wallet` | No | Address to check (defaults to logged-in wallet) |

**Example:**
```bash
moonwell positions --chain 8453
moonwell positions --wallet 0xYourAddress
```

---

### `supply` — Supply Asset to Earn Interest

Deposit an asset into Moonwell to receive mTokens and earn supply APR + WELL rewards.

**Usage:**
```
moonwell supply --asset <SYMBOL> --amount <AMOUNT> [--chain 8453] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--asset` | Yes | Asset symbol: USDC, WETH, cbETH, USDbC, DAI |
| `--amount` | Yes | Amount to supply (e.g. `0.01` for 0.01 USDC) |
| `--chain` | No | Chain ID (default: 8453) |
| `--from` | No | Wallet address |
| `--dry-run` | No | Simulate without broadcasting |

**WARNING:** **Ask user to confirm** before submitting. Two transactions are required: (1) ERC20 approve, (2) mToken.mint.

**Steps:**
1. `ERC20.approve(mToken, amount)` — allow mToken to pull funds
2. `mToken.mint(amount)` — deposit and receive mTokens

**Example:**
```bash
moonwell supply --asset USDC --amount 0.01 --chain 8453
moonwell supply --asset USDC --amount 0.01 --chain 8453 --dry-run
```

---

### `redeem` — Redeem mTokens

Burn mTokens to withdraw your underlying asset (principal + accrued interest).

**Usage:**
```
moonwell redeem --asset <SYMBOL> --mtoken-amount <AMOUNT> [--chain 8453] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--asset` | Yes | Asset symbol: USDC, WETH, cbETH, USDbC, DAI |
| `--mtoken-amount` | Yes | mToken amount to redeem (8 decimal precision) |
| `--chain` | No | Chain ID (default: 8453) |
| `--from` | No | Wallet address |
| `--dry-run` | No | Simulate without broadcasting |

**WARNING:** **Ask user to confirm** before submitting.

**Example:**
```bash
moonwell redeem --asset USDC --mtoken-amount 100.5 --chain 8453
moonwell redeem --asset USDC --mtoken-amount 100.5 --chain 8453 --dry-run
```

---

### `borrow` — Borrow Asset (Dry-Run Only)

Preview borrowing an asset against your supplied collateral. **Real execution disabled for safety.**

**Usage:**
```
moonwell borrow --asset <SYMBOL> --amount <AMOUNT> [--chain 8453] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--asset` | Yes | Asset symbol: USDC, WETH, cbETH, USDbC, DAI |
| `--amount` | Yes | Amount to borrow |
| `--chain` | No | Chain ID (default: 8453) |
| `--dry-run` | Yes | Required — borrow only runs in dry-run mode |

**WARNING:** Borrowing requires sufficient collateral. Under-collateralization leads to **liquidation**. This command is dry-run only.

**Example:**
```bash
moonwell borrow --asset USDC --amount 5.0 --chain 8453 --dry-run
```

---

### `repay` — Repay Borrow (Dry-Run Only)

Preview repaying an outstanding borrow position. **Real execution disabled for safety.**

**Usage:**
```
moonwell repay --asset <SYMBOL> --amount <AMOUNT> [--chain 8453] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--asset` | Yes | Asset symbol: USDC, WETH, cbETH, USDbC, DAI |
| `--amount` | Yes | Amount to repay |
| `--chain` | No | Chain ID (default: 8453) |
| `--dry-run` | Yes | Required — repay only runs in dry-run mode |

**WARNING:** **Ask user to confirm** before submitting. Dry-run only for safety.

**Example:**
```bash
moonwell repay --asset USDC --amount 5.0 --chain 8453 --dry-run
```

---

### `claim-rewards` — Claim WELL Rewards

Claim all accrued WELL token rewards from the Moonwell Comptroller.

**Usage:**
```
moonwell claim-rewards [--chain 8453] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--chain` | No | Chain ID (default: 8453) |
| `--from` | No | Wallet address |
| `--dry-run` | No | Simulate without broadcasting |

**WARNING:** **Ask user to confirm** before submitting.

**Example:**
```bash
moonwell claim-rewards --chain 8453
moonwell claim-rewards --chain 8453 --dry-run
```

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "Could not resolve wallet address" | Not logged in | Run `onchainos wallet login` |
| "Unknown asset 'X'" | Invalid symbol | Use: USDC, WETH, cbETH, USDbC, DAI |
| "borrow is only available in --dry-run" | Missing --dry-run flag | Add `--dry-run` flag |
| "repay is only available in --dry-run" | Missing --dry-run flag | Add `--dry-run` flag |
| "Chain X is not supported" | Wrong chain ID | Use chain 8453 (Base) |
| "RPC error" | Node connectivity issue | Retry; check network |

## Risk Warnings

- Borrowing creates liquidation risk if collateral value falls
- Always check your account liquidity before borrowing: use `positions` command
- Never borrow more than 70% of your collateral value
- Borrow and repay operations are dry-run only in this plugin

## Suggested Follow-ups

After **markets**: suggest checking your `moonwell positions` to see existing exposure.

After **positions** (has supply, no borrow): mention that you can borrow against collateral.

After **supply**: suggest using `moonwell positions` to verify the deposit was recorded, or `moonwell claim-rewards` to claim pending WELL rewards.

After **redeem**: suggest checking `moonwell positions` to confirm withdrawal.

After **claim-rewards**: mention that WELL tokens can be staked in stkWELL for additional yield.

## Skill Routing

- For ETH staking → use `lido` or `etherfi-stake` skill
- For wallet balance → use `onchainos wallet balance --chain 8453`
- For other lending on Base → use `aave-v3` skill
