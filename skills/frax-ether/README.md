# Frax Ether Plugin

Frax Ether liquid staking integration for onchainos. Stake ETH to receive frxETH, then stake frxETH to earn yield as sfrxETH.

## Supported Chain

- Ethereum mainnet (chain ID: 1)

## Commands

| Command | Description |
|---------|-------------|
| `stake --amount <eth>` | Stake ETH → frxETH via frxETHMinter |
| `stake-frx --amount <frxeth>` | Stake frxETH → sfrxETH (ERC-4626 deposit) |
| `unstake --amount <sfrxeth>` | Redeem sfrxETH → frxETH (ERC-4626 redeem) |
| `rates` | Get current sfrxETH APR and exchange rate |
| `positions [--address <addr>]` | Query frxETH + sfrxETH balances |

## Key Contracts

- **frxETHMinter**: `0xbAFA44EFE7901E04E39Dad13167D089C559c1138`
- **frxETH**: `0x5E8422345238F34275888049021821E8E08CAa1f`
- **sfrxETH**: `0xac3E018457B222d93114458476f3E3416Abbe38F`
