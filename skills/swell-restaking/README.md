# Swell Restaking Plugin

Stake ETH on Swell Network to receive **rswETH** - a liquid restaking token that earns both Ethereum validator rewards and EigenLayer AVS restaking rewards.

## Supported Chain

- Ethereum mainnet (chain ID: 1)

## Commands

| Command | Description |
|---|---|
| `get-rates` | Get current rswETH exchange rates and pool stats |
| `get-positions` | View rswETH balance and ETH-equivalent value |
| `stake` | Stake ETH to receive rswETH |

## Quick Start

```bash
# Get rswETH exchange rates
swell-restaking get-rates --chain 1

# Check your rswETH balance
swell-restaking get-positions --chain 1

# Stake 0.00005 ETH (dry-run first)
swell-restaking stake --amount 0.00005 --chain 1 --dry-run

# Stake for real (asks for confirmation)
swell-restaking stake --amount 0.00005 --chain 1
```

## Key Facts

- rswETH appreciates in value vs ETH (non-rebasing) as rewards accumulate
- Earns both validator rewards AND EigenLayer AVS restaking rewards
- Unstaking requires using the Swell app (https://app.swellnetwork.io) with 1-12 day queue
- rswETH contract: `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0`

## Related

- For Swell liquid staking (swETH): use the `swell-staking` plugin
- For Lido staking (stETH): use the `lido` plugin
