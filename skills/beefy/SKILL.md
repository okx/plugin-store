---
name: beefy
description: "Beefy Finance yield optimizer - deposit into auto-compounding vaults on Base, BSC, and other EVM chains. Trigger phrases: beefy vaults, beefy apy, deposit to beefy, beefy yield, my beefy positions, withdraw from beefy, beefy finance, mooToken, auto-compound, beefy base, beefy bsc"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Beefy Finance Skill

Interact with Beefy Finance yield optimizer vaults. Beefy auto-compounds LP and farming rewards so your position grows over time.

Supported chains: Base (8453), BSC (56), Ethereum (1), Polygon (137), Arbitrum (42161), Optimism (10)

## Commands

### Read Commands (safe, no wallet needed)

#### `vaults`
List active Beefy vaults with APY and TVL.

```
beefy vaults --chain 8453
beefy vaults --chain 8453 --asset USDC
beefy vaults --chain 8453 --platform morpho
beefy vaults --chain 56 --limit 10
```

#### `apy`
Show APY rates for Beefy vaults on a chain.

```
beefy apy --chain 8453
beefy apy --chain 8453 --asset USDC
beefy apy --chain 8453 --vault morpho-base-gauntlet-prime-usdc
```

#### `positions`
View your mooToken balances across all active Beefy vaults.

```
beefy positions --chain 8453
beefy positions --chain 8453 --wallet 0xYourAddress
```

### Write Commands (require wallet confirmation)

> **IMPORTANT**: Before executing deposit or withdraw, always ask the user to confirm
> the transaction details - vault, amount, and chain. These operations move real funds.

#### `deposit`
Deposit tokens into a Beefy vault to start auto-compounding.

**Steps**: (1) ERC-20 approve vault for spending - (2) ERC-4626 deposit(amount, receiver)

```
beefy deposit --vault morpho-base-gauntlet-prime-usdc --amount 0.01 --chain 8453
beefy deposit --vault morpho-base-gauntlet-prime-usdc --amount 0.01 --chain 8453 --dry-run
beefy deposit --vault aerodrome-weth-usdc --amount 0.01 --chain 8453
```

#### `withdraw`
Redeem mooTokens to get your underlying tokens back.

```
beefy withdraw --vault morpho-base-gauntlet-prime-usdc --chain 8453
beefy withdraw --vault morpho-base-gauntlet-prime-usdc --shares 0.5 --chain 8453 --dry-run
```

## Notes

- Beefy vaults issue mooTokens representing your share
- pricePerFullShare increases over time as rewards compound
- Vault IDs follow pattern: `{platform}-{assets}` (e.g. `morpho-base-gauntlet-prime-usdc`)
- Use `vaults` command to find the vault ID you need
- Status `eol` means the vault is retired - no new deposits accepted
- USDC on Base: `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`
