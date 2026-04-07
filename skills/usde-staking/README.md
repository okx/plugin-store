# USDe Staking Plugin (Ethena sUSDe)

Stake USDe to earn yield via Ethena's sUSDe ERC-4626 vault on Ethereum mainnet.

## Features

- **Stake**: Deposit USDe to receive sUSDe (earns yield automatically)
- **Request Unstake**: Initiate cooldown to convert sUSDe back to USDe
- **Claim Unstake**: Receive USDe after cooldown period
- **Get Rates**: Current APY, 30/90-day averages, exchange rate
- **Get Positions**: Your sUSDe balance, USDe equivalent, pending cooldowns

## Supported Chains

- Ethereum mainnet (chain ID 1)

## Key Contracts

| Contract | Address |
|---|---|
| USDe token | `0x4c9EDD5852cd905f086C759E8383e09bff1E68B3` |
| sUSDe vault | `0x9D39A5DE30e57443BfF2A8307A4256c8797A3497` |

## Usage

```bash
# Check current APY
usde-staking get-rates

# View your position
usde-staking get-positions

# Stake USDe (preview first)
usde-staking stake --amount 100.0 --dry-run
usde-staking stake --amount 100.0

# Request unstake (starts cooldown)
usde-staking request-unstake --shares 50.0 --dry-run
usde-staking request-unstake --shares 50.0

# Claim after cooldown
usde-staking claim-unstake
```
