
# frax-ether -- Skill Summary

## Overview
This skill provides comprehensive integration with the Frax Ether liquid staking protocol on Ethereum mainnet. It enables users to stake ETH to receive frxETH tokens, then stake those frxETH tokens into the sfrxETH ERC-4626 vault to earn staking yield. The skill handles the complete two-step staking flow, from initial ETH deposits through yield-bearing vault operations, while providing real-time rate information and portfolio tracking capabilities.

## Usage
Install the plugin and ensure onchainos CLI is available, then use commands like `frax-ether stake --amount 0.1` to stake ETH or `frax-ether rates` to check current yields. All write operations require explicit user confirmation before broadcasting transactions.

## Commands
| Command | Description |
|---------|-------------|
| `stake --amount <eth>` | Stake ETH → frxETH via frxETHMinter |
| `stake-frx --amount <frxeth>` | Stake frxETH → sfrxETH (ERC-4626 deposit) |
| `unstake --amount <sfrxeth>` | Redeem sfrxETH → frxETH (ERC-4626 redeem) |
| `rates` | Get current sfrxETH APR and exchange rate |
| `positions [--address <addr>]` | Query frxETH + sfrxETH balances |

## Triggers
Activate this skill when users want to stake ETH through Frax protocol, earn yield on frxETH holdings, check sfrxETH rates and APR, or manage their liquid staking positions. Trigger phrases include "stake ETH frax", "frxETH yield", "sfrxETH position", and "frax liquid staking".
