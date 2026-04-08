# Kelp DAO rsETH Plugin

Kelp DAO liquid restaking plugin for onchainos. Stake ETH or LSTs (stETH, ETHx, sfrxETH) to receive **rsETH** — a Liquid Restaking Token built on EigenLayer.

## Features

- **apy** — Query current rsETH staking yield
- **rates** — Get rsETH/ETH exchange rate from LRTOracle on-chain
- **positions** — View rsETH balance and underlying ETH value
- **stake** — Deposit ETH → rsETH via LRTDepositPool
- **unstake** — Initiate rsETH → ETH withdrawal via LRTWithdrawalManager

## Supported Chains

| Chain | Chain ID | Support |
|---|---|---|
| Ethereum | 1 | Full (deposit, withdraw, oracle) |
| Base | 8453 | rsETH bridged (balance query) |
| Arbitrum | 42161 | rsETH bridged (balance query) |

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|---|---|
| rsETH Token | `0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7` |
| LRTDepositPool | `0x036676389e48133B63a802f8635AD39E752D375D` |
| LRTOracle | `0x349A73444b1a310BAe67ef67973022020d70020d` |
| LRTWithdrawalManager | `0x62De59c08eB5dAE4b7E6F7a8cAd3006d6965ec16` |

## Usage

```bash
# Check current APY
kelp apy

# Get rsETH/ETH exchange rate
kelp rates --chain 1

# Check your rsETH balance
kelp positions --chain 1

# Stake 0.1 ETH (dry run first)
kelp stake --amount 0.1 --chain 1 --dry-run
kelp stake --amount 0.1 --chain 1

# Unstake rsETH
kelp unstake --amount 0.05 --chain 1
```

## Building

```bash
cargo build --release
```

## License

MIT
