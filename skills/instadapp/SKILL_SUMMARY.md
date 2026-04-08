
# instadapp -- Skill Summary

## Overview
This skill provides comprehensive access to Instadapp Lite vaults on Ethereum, enabling users to deposit ETH or stETH into yield-generating vaults, track their positions, monitor exchange rates, and withdraw funds. It supports both the iETH v1 vault (accepting native ETH) and the iETHv2 vault (accepting stETH via ERC-4626), which generate yield through leveraged stETH/WETH positions across multiple DeFi protocols including Aave, Compound, Spark, and Fluid.

## Usage
Install the binary and ensure onchainos wallet is configured for Ethereum mainnet. Use commands like `instadapp vaults` to view available vaults, `instadapp deposit --vault v1 --amount 0.0001` to deposit ETH, and `instadapp positions` to check holdings.

## Commands
| Command | Description |
|---------|-------------|
| `instadapp vaults` | List available Instadapp Lite vaults with TVL and exchange prices |
| `instadapp rates` | Show yield rates and exchange price details |
| `instadapp positions` | Query your iETH/iETHv2 share balances and holdings |
| `instadapp deposit --vault v1 --amount <amount>` | Deposit ETH into iETH v1 vault or stETH into iETHv2 |
| `instadapp withdraw --vault v1` | Withdraw by redeeming iETH/iETHv2 shares |

## Triggers
Activate this skill when users mention Instadapp, Instadapp Lite, iETH vault, iETHv2, or want to deposit/withdraw ETH for yield farming on Ethereum. Also trigger for queries about Instadapp positions, rates, or yield tracking.
