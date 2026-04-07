# yearn-finance

Yearn Finance yVaults plugin for onchainos Plugin Store.

Deposit, withdraw, and track auto-compounding yield from Yearn yVaults (ERC-4626) on Ethereum mainnet.

## Commands

- `vaults` — List active Yearn vaults with APR and TVL
- `rates` — Show detailed APR history
- `positions` — Query your vault share holdings
- `deposit` — Deposit ERC-20 tokens into a vault
- `withdraw` — Redeem shares from a vault

## Usage

```bash
yearn-finance vaults --token USDT
yearn-finance rates
yearn-finance positions
yearn-finance deposit --vault USDT --amount 0.01 --dry-run
yearn-finance withdraw --vault USDT
```

## Supported Chain

- Ethereum mainnet (chain ID 1)

## Data Sources

- [yDaemon API](https://ydaemon.yearn.fi) — vault metadata, APR, TVL
- [Ethereum publicnode RPC](https://ethereum.publicnode.com) — on-chain balance queries
