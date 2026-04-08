# euler-v2

Euler V2 plugin for onchainos — interact with modular ERC-4626 lending vaults (EVaults) across EVM chains.

## Features

- **markets** — list lending markets with TVL and borrow rates
- **positions** — view your supply/borrow positions
- **supply** — deposit assets into EVaults (ERC-4626 deposit)
- **withdraw** — withdraw assets from EVaults
- **borrow** — simulate borrow calldata (dry-run only)
- **repay** — simulate repay calldata (dry-run only)

## Supported Chains

Base (8453), Ethereum (1), Arbitrum (42161), Avalanche (43114), BSC (56)

## Usage

```bash
# List markets on Base
euler-v2 --chain 8453 markets

# View positions
euler-v2 --chain 8453 positions

# Supply 10 USDC (confirms before executing)
euler-v2 --chain 8453 supply --vault USDC --amount 10

# Withdraw 5 USDC
euler-v2 --chain 8453 withdraw --vault USDC --amount 5

# Simulate borrow (dry-run only)
euler-v2 --chain 8453 --dry-run borrow --vault USDC --amount 100

# Simulate repay (dry-run only)
euler-v2 --chain 8453 --dry-run repay --vault USDC --all
```

## Architecture

Euler V2 uses the Ethereum Vault Connector (EVC) as a central coordinator for batch operations and cross-vault interactions. Each EVault is ERC-4626 compliant with additional borrow/repay functions.
