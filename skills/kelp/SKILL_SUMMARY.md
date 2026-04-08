
# kelp -- Skill Summary

## Overview
The kelp plugin provides comprehensive liquid restaking functionality for Kelp DAO's rsETH protocol built on EigenLayer. Users can stake ETH or liquid staking tokens to receive rsETH, which accrues both EigenLayer restaking rewards and underlying staking yields. The plugin handles deposits, withdrawals, balance queries, rate checking, and APY monitoring across Ethereum mainnet and bridged chains like Base and Arbitrum.

## Usage
Install the plugin via the plugin store, then use commands like `kelp stake --amount 0.1` to deposit ETH for rsETH tokens. All write operations require user confirmation before broadcasting transactions to ensure safety.

## Commands
| Command | Description |
|---------|-------------|
| `kelp apy` | Get current rsETH estimated APY from yield sources |
| `kelp rates [--chain <ID>]` | Get rsETH/ETH exchange rate from on-chain oracle |
| `kelp positions [--address <ADDR>] [--chain <ID>]` | Check rsETH balance and ETH value |
| `kelp stake --amount <ETH> [--chain <ID>] [--dry-run]` | Deposit ETH to receive rsETH via LRTDepositPool |
| `kelp unstake --amount <RSETH> [--chain <ID>] [--dry-run]` | Initiate rsETH withdrawal via LRTWithdrawalManager |

## Triggers
Activate this skill when users want to participate in liquid restaking on EigenLayer, stake ETH for rsETH tokens, check restaking yields, or manage existing rsETH positions. Use when users mention Kelp DAO, rsETH, liquid restaking, or EigenLayer restaking rewards.
