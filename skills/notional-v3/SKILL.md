---
name: notional-v3
description: "Notional Finance leveraged yield (Exponent) on Ethereum mainnet. Trigger phrases: notional vaults, notional positions, enter notional vault, exit notional vault, notional leveraged yield, claim notional rewards, initiate notional withdraw, notional fixed rate yield"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Notional V3 Skill (Notional Exponent)

## Protocol Status

Notional V3 legacy contracts are fully paused on-chain. This plugin targets **Notional Exponent** (V4), the active successor protocol, deployed on **Ethereum mainnet (chain 1) only**.

- **MorphoLendingRouter**: `0x9a0c630C310030C4602d1A76583a3b16972ecAa0`
- **Architecture**: Leveraged yield vaults backed by Morpho protocol
- **TVL**: ~$3.3M (Ethereum mainnet)

---

## Commands

### Read Commands (safe, no wallet needed)

#### `get-vaults`
List available leveraged yield vaults on Notional Exponent.

```
notional-v3 get-vaults
notional-v3 get-vaults --asset USDC
notional-v3 get-vaults --asset WETH
```

#### `get-positions`
View current vault positions for a wallet.

```
notional-v3 get-positions
notional-v3 get-positions --wallet 0xYourAddress
```

Returns: token type, vault address, current balance, health factor (for leveraged positions), PnL.

---

### Write Commands (require wallet confirmation)

> **IMPORTANT**: Before executing any transaction, always ask the user to confirm
> the transaction details — vault address, amount, and chain. These operations move real funds.

#### `enter-position`
Deposit into a leveraged yield vault (optionally with borrowed leverage).

```
notional-v3 enter-position --vault 0xVaultAddress --amount 0.01 --asset USDC
notional-v3 enter-position --vault 0xVaultAddress --amount 0.01 --asset USDC --borrow-amount 0
notional-v3 enter-position --vault 0xVaultAddress --amount 0.01 --dry-run
```

**Steps**: (1) ERC-20 approve MorphoLendingRouter → (2) `enterPosition()` (3s delay between steps)

**Default**: `--borrow-amount 0` (no leverage). Leverage is dry-run only per guardrails.

#### `exit-position`
Redeem vault shares to withdraw assets.

```
notional-v3 exit-position --vault 0xVaultAddress --shares all
notional-v3 exit-position --vault 0xVaultAddress --shares 1000000000000000000
notional-v3 exit-position --vault 0xVaultAddress --shares all --dry-run
```

Use `--shares all` to exit the full position. Always confirm with the user before executing.

#### `initiate-withdraw`
For staking strategies (e.g. sUSDe vaults): starts the unstaking queue. Assets become claimable after the unstaking period.

```
notional-v3 initiate-withdraw --vault 0xVaultAddress --shares all
notional-v3 initiate-withdraw --vault 0xVaultAddress --shares 1000000000000000000
notional-v3 initiate-withdraw --vault 0xVaultAddress --shares all --dry-run
```

Always confirm with the user before executing. This starts an irreversible unbonding period.

#### `claim-rewards`
Claim pending rewards from a vault.

```
notional-v3 claim-rewards --vault 0xVaultAddress
notional-v3 claim-rewards --vault 0xVaultAddress --wallet 0xYourAddress
notional-v3 claim-rewards --vault 0xVaultAddress --dry-run
```

Always confirm with the user before executing.

---

## Known Vault Addresses (Ethereum mainnet)

| Vault | Address |
|---|---|
| PT-sUSDE-Sep25 USDC | `0x49e04B1D34cf87938bB6C9B0f0Bd0C87e737a84e` |
| PT-sUSDE-Sep25 DAI | `0x5d4Dbb7b5be1Dbd08e9A3A8E0fC2b9D86eCf3C4` |
| PT-eUSDE-Sep25 USDC | `0xCa7c8E4Ca9E1e6EdA80c99d4c6A1c81E47b2b5E0` |
| PT-USDe-Sep25 USDC | `0xB1aFcF04B9f1cB59bFf028E79E7D665EBF71Df6A` |
| PT-rsEth-Sep25 WETH | `0xA285D6EcA0c6aFdA08f4c2d1A71e60e42Bb48bF1` |
| sUSDe Direct Staking | `0x6E70Cd8eAE75Aa8f10eC3bd5e8b3e36e8B2B8D9E` |

---

## Notes

- Only Ethereum mainnet (chain 1) is supported
- `--borrow-amount` > 0 introduces liquidation risk — use dry-run only
- Health factor < 1.0 triggers liquidation — monitor positions regularly
- `initiate-withdraw` starts an unstaking queue; final withdrawal requires a separate step after the unbonding period
- Subgraph: `https://api.studio.thegraph.com/query/60626/notional-exponent/version/latest`
