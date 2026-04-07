# morpho

Supply, borrow and earn yield on [Morpho](https://morpho.org/) — a permissionless lending protocol with $5B+ TVL, supporting both Morpho Blue isolated markets and MetaMorpho vaults.

## Features

- **Supply** assets to MetaMorpho vaults and earn yield
- **Withdraw** from MetaMorpho vaults (partial or full)
- **Borrow** from Morpho Blue isolated markets
- **Repay** Morpho Blue debt (partial or full, dust-free)
- **Supply collateral** to Morpho Blue markets
- **View positions** with health factors across Blue markets and MetaMorpho vaults
- **Browse markets** with supply/borrow APYs and utilization rates
- **Browse vaults** with APYs and curators (Gauntlet, Steakhouse, etc.)
- **Claim rewards** via Merkl distributor

## Supported Chains

| Chain | Chain ID |
|-------|----------|
| Ethereum Mainnet | 1 (default) |
| Base | 8453 |

## Install

```bash
npx skills add okx/plugin-store-community --skill morpho
```

## Usage Examples

```bash
# View your positions
morpho positions

# List USDC markets on Base
morpho --chain 8453 markets --asset USDC

# List MetaMorpho vaults on Ethereum
morpho vaults --asset WETH

# Supply to a vault (dry-run first)
morpho --dry-run supply --vault 0xBEEF01735c132Ada46AA9aA4c54623cAA92A64CB --asset USDC --amount 1000

# Borrow from Morpho Blue (dry-run first)
morpho --dry-run borrow --market-id 0xb323495f7e4148be5643a4ea4a8221eef163e4bccfdedc2a6f4696baacbc86cc --amount 500

# Claim rewards
morpho claim-rewards
```

## Architecture

- **Read operations** (positions, markets, vaults) — queries `https://blue-api.morpho.org/graphql` directly
- **Write operations** (supply, withdraw, borrow, repay, supply-collateral, claim-rewards) — submits signed transactions via `onchainos wallet contract-call` after user confirmation
- **Safety**: always dry-runs first, shows transaction details, requires explicit confirmation before broadcasting

## Source

- Plugin Store entry: [okx/plugin-store-community](https://github.com/okx/plugin-store-community/tree/main/submissions/morpho)

## License

MIT
