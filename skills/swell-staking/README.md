# swell-staking

Swell Network liquid staking plugin for onchainos. Stake ETH to receive swETH or rswETH on Ethereum mainnet.

## Supported Operations

| Command | Description |
|---------|-------------|
| `rates` | Get current swETH/rswETH exchange rates |
| `positions` | View swETH and rswETH holdings |
| `stake` | Stake ETH → swETH (liquid staking) |
| `restake` | Restake ETH → rswETH (EigenLayer liquid restaking) |

## Usage

```bash
# Get exchange rates
swell-staking rates

# Check positions
swell-staking positions --address 0xYourAddress

# Stake ETH
swell-staking stake --amount 0.001

# Restake ETH (EigenLayer)
swell-staking restake --amount 0.001

# Dry run
swell-staking stake --amount 0.001 --dry-run
```

## Contracts (Ethereum Mainnet)

- **swETH**: `0xf951E335afb289353dc249e82926178EaC7DEd78`
- **rswETH**: `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0`
