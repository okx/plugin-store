# Rocket Pool Plugin

Decentralised ETH liquid staking via [Rocket Pool](https://rocketpool.net). Stake ETH to receive rETH — a non-rebasing liquid staking token that appreciates in value relative to ETH as staking rewards accumulate.

## Features

- `rate` — current ETH/rETH exchange rate
- `apy` — current staking APY
- `stats` — protocol stats: TVL, nodes, minipools
- `positions` — user's rETH balance and ETH value
- `stake` — deposit ETH → receive rETH
- `unstake` — burn rETH → receive ETH

## Usage

```bash
rocket-pool rate --chain 1
rocket-pool apy --chain 1
rocket-pool stats --chain 1
rocket-pool positions --chain 1
rocket-pool stake --amount 0.05 --chain 1
rocket-pool unstake --amount 0.05 --chain 1
```

## Chain Support

- Ethereum Mainnet (chain ID: 1)

## Minimum Deposit

The Rocket Pool protocol enforces a minimum deposit of **0.01 ETH**.

## Architecture

Contract addresses are resolved dynamically via `RocketStorage` (`0x1d8f8f00cfa6758d7bE78336684788Fb0ee0Fa46`), ensuring the plugin works correctly even after Rocket Pool contract upgrades.
