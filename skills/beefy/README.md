# Beefy Finance Plugin

Beefy Finance yield optimizer integration for onchainos. Deposit into auto-compounding vaults on Base, BSC, Ethereum, and other EVM chains.

## Features

- List active vaults with APY and TVL
- View your mooToken positions
- Deposit tokens into vaults (ERC-4626)
- Withdraw (redeem mooTokens)

## Supported Chains

- Base (8453) - primary
- BSC (56)
- Ethereum (1)
- Polygon (137)
- Arbitrum (42161)
- Optimism (10)

## Quick Start

```bash
# List Base vaults
beefy vaults --chain 8453

# Find USDC vaults on Base
beefy vaults --chain 8453 --asset USDC

# Check APY
beefy apy --chain 8453 --asset USDC

# Check your positions
beefy positions --chain 8453

# Deposit (simulation first)
beefy deposit --vault morpho-base-gauntlet-prime-usdc --amount 0.01 --chain 8453 --dry-run

# Actual deposit
beefy deposit --vault morpho-base-gauntlet-prime-usdc --amount 0.01 --chain 8453
```
