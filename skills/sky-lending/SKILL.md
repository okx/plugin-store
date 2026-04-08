---
name: sky-lending
version: 0.1.0
description: "Sky Lending (MakerDAO CDP) — deposit collateral, borrow DAI/USDS stablecoins on Ethereum"
chains:
  - ethereum
category: defi-protocol
tags:
  - lending
  - cdp
  - dai
  - makerdao
  - sky
  - stablecoin
  - collateral
author: GeoGu360
---

# Sky Lending Plugin (MakerDAO CDP)

## Overview

Sky Lending is the rebrand of MakerDAO's CDP (Collateralized Debt Position) system. Users deposit collateral into vaults and borrow DAI stablecoins against it. The protocol runs on Ethereum mainnet (chain ID 1).

**Key facts:**
- Sky = MakerDAO rebrand; core contracts (Vat, DssCdpManager) remain unchanged
- CDP = Collateralized Debt Position: lock collateral → draw DAI debt
- Liquidation risk: if collateral value falls below the liquidation ratio, the vault gets liquidated
- Stability fees accrue continuously on debt (currently ~2-5% APY depending on collateral type)
- All write operations in this plugin are dry-run only for safety

## Pre-flight Checks

Before any command:
1. Verify `onchainos` is installed: `onchainos --version`
2. For operations requiring wallet: `onchainos wallet balance --chain 1 --output json`
3. If wallet check fails: "Please log in with `onchainos wallet login` first."

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|---|---|
| DssCdpManager | `0x5ef30b9986345249bc32d8928B7ee64DE9435E39` |
| Vat (core accounting) | `0x35D1b3F3D7966A1DFe207aa4514C12a259A0492B` |
| Jug (stability fees) | `0x19c0976f590D67707E62397C87829d896Dc0f1F` |
| ETH-A Join | `0x2F0b23f53734252Bda2277357e97e1517d6B042A` |
| DAI Join | `0x9759A6Ac90977b93B58547b4A71c78317f391A28` |
| DAI Token | `0x6B175474E89094C44Da98b954EedeAC495271d0F` |

---

## Commands

### `ilks` — List Collateral Types

Query on-chain parameters for all supported collateral types.

**Usage:**
```
sky-lending ilks [--chain 1]
```

**Returns per ilk:** total debt, rate accumulator, spot price, debt ceiling, minimum vault debt, and approximate stability fee APY.

**Supported ilks:** ETH-A, WBTC-A, USDC-A, WSTETH-A

**Example:**
```bash
sky-lending ilks --chain 1
```

---

### `vaults` — List CDP Vaults

Query all CDP vaults for a given address.

**Usage:**
```
sky-lending vaults [--chain 1] [--address <ADDR>]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--chain` | No | Chain ID (default: 1) |
| `--address` | No | Owner address (resolved from onchainos if omitted) |

**Returns per vault:** CDP ID, urn address, ilk type, collateral amount (ink), DAI debt (art * rate).

**Example:**
```bash
sky-lending vaults --chain 1
sky-lending vaults --address 0xYourAddress
```

---

### `open-vault` — Open New CDP Vault (Dry-Run)

Generate calldata to open a new CDP vault.

**Usage:**
```
sky-lending open-vault [--chain 1] [--ilk ETH-A] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--chain` | No | Chain ID (default: 1) |
| `--ilk` | No | Collateral type (default: ETH-A) |
| `--from` | No | Wallet address |
| `--dry-run` | No | Default true — always dry-run for safety |

**WARNING:** **Ask user to confirm** before submitting any real transaction. CDP operations carry liquidation risk.

**Example:**
```bash
sky-lending open-vault --ilk ETH-A --dry-run
```

---

### `deposit-collateral` — Deposit ETH Collateral (Dry-Run)

Generate calldata to lock ETH collateral into a vault.

**Usage:**
```
sky-lending deposit-collateral --amount-eth <ETH> [--urn <URN>] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount-eth` | Yes | ETH amount to deposit (e.g. `1.5`) |
| `--urn` | No | Vault urn address (from `sky-lending vaults`) |
| `--from` | No | Wallet address |
| `--dry-run` | No | Default true — always dry-run for safety |

**WARNING:** **Ask user to confirm** before submitting. Depositing incorrect amounts or to wrong vaults is irreversible.

**Steps:**
1. Calls `GemJoin.join(address urn)` with ETH value (payable)
2. ETH is locked in the Vat as collateral

**Example:**
```bash
sky-lending deposit-collateral --amount-eth 1.0 --urn 0xUrnAddress --dry-run
```

---

### `draw-dai` — Draw (Mint) DAI (Dry-Run)

Generate calldata to borrow DAI against collateral.

**Usage:**
```
sky-lending draw-dai --amount-dai <DAI> [--ilk ETH-A] [--urn <URN>] [--to <ADDR>] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount-dai` | Yes | DAI amount to draw (e.g. `500.0`) |
| `--ilk` | No | Collateral type (default: ETH-A) |
| `--urn` | No | Vault urn address |
| `--to` | No | Recipient address for DAI |
| `--from` | No | Wallet address |
| `--dry-run` | No | Default true — always dry-run for safety |

**WARNING:** **Ask user to confirm** before submitting. Drawing too much DAI risks liquidation if collateral price drops.

**Steps:**
1. `Vat.frob(ilk, urn, urn, urn, 0, dart)` — increase debt in Vat
2. `DaiJoin.exit(recipient, amount)` — mint DAI to wallet

**Example:**
```bash
sky-lending draw-dai --amount-dai 500.0 --ilk ETH-A --dry-run
```

---

### `repay-dai` — Repay DAI Debt (Dry-Run)

Generate calldata to repay DAI debt.

**Usage:**
```
sky-lending repay-dai --amount-dai <DAI> [--ilk ETH-A] [--urn <URN>] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount-dai` | Yes | DAI amount to repay (e.g. `100.0`) |
| `--ilk` | No | Collateral type (default: ETH-A) |
| `--urn` | No | Vault urn address |
| `--from` | No | Wallet address |
| `--dry-run` | No | Default true — always dry-run for safety |

**WARNING:** **Ask user to confirm** before submitting. Ensure sufficient DAI balance first.

**Steps:**
1. `DAI.approve(DaiJoin, amount)` — allow DaiJoin to pull DAI
2. `DaiJoin.join(urn, amount)` — burn DAI, credit Vat
3. `Vat.frob(ilk, urn, urn, urn, 0, -dart)` — reduce debt

**Example:**
```bash
sky-lending repay-dai --amount-dai 100.0 --ilk ETH-A --dry-run
```

---

### `withdraw-collateral` — Withdraw Collateral (Dry-Run)

Generate calldata to withdraw ETH collateral from a vault.

**Usage:**
```
sky-lending withdraw-collateral --amount-eth <ETH> [--ilk ETH-A] [--urn <URN>] [--to <ADDR>] [--from <ADDR>] [--dry-run]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount-eth` | Yes | ETH amount to withdraw (e.g. `0.5`) |
| `--ilk` | No | Collateral type (default: ETH-A) |
| `--urn` | No | Vault urn address |
| `--to` | No | Recipient for withdrawn ETH |
| `--from` | No | Wallet address |
| `--dry-run` | No | Default true — always dry-run for safety |

**WARNING:** **Ask user to confirm** before submitting. Withdrawing collateral lowers collateralization ratio and increases liquidation risk.

**Steps:**
1. `Vat.frob(ilk, urn, urn, urn, -dink, 0)` — free collateral in Vat
2. `GemJoin.exit(recipient, amount)` — withdraw ETH to wallet

**Example:**
```bash
sky-lending withdraw-collateral --amount-eth 0.5 --ilk ETH-A --dry-run
```

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "Cannot get wallet address" | Not logged in to onchainos | Run `onchainos wallet login` |
| "Unknown ilk: X" | Invalid collateral type | Use: ETH-A, WBTC-A, USDC-A, WSTETH-A |
| "eth_call RPC error" | RPC node issue | Retry; check network |
| "Return data too short" | Contract returned empty | Verify contract address and chain ID |

## Risk Warnings

- CDP operations carry real liquidation risk in production
- All write operations in this plugin are dry-run only
- Never submit CDP transactions without verifying your collateralization ratio
- Minimum collateralization ratios: ETH-A 150%, WBTC-A 150%, WSTETH-A 160%

## Suggested Follow-ups

After **ilks**: suggest viewing your vaults with `sky-lending vaults`.

After **vaults**: if vault has debt, suggest checking collateralization ratio; if empty, suggest `sky-lending deposit-collateral`.

After **open-vault** (dry-run): suggest `sky-lending deposit-collateral` to add collateral.

After **deposit-collateral** (dry-run): suggest `sky-lending draw-dai` to borrow DAI.

After **draw-dai** (dry-run): remind user to monitor their vault health regularly.

## Skill Routing

- For ETH staking → use `lido` skill
- For savings rate on DAI → use `spark-savings` skill
- For wallet balance → use `onchainos wallet balance`
