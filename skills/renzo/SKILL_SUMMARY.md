
# renzo -- Skill Summary

## Overview
This plugin enables interaction with the Renzo liquid restaking protocol on Ethereum mainnet, allowing users to deposit ETH or stETH to receive ezETH (liquid restaking tokens) and earn EigenLayer AVS rewards. It provides both read operations (balance checks, APR queries, TVL monitoring) and write operations (deposits) with mandatory user confirmation for all transactions.

## Usage
Install via `npx skills add okx/plugin-store --skill renzo` and ensure onchainos wallet is connected. All write operations require `--confirm` flag after previewing transaction details.

## Commands
| Command | Description |
|---------|-------------|
| `renzo deposit-eth --amount-eth <ETH_AMOUNT>` | Deposit native ETH to mint ezETH |
| `renzo deposit-steth --amount <STETH_AMOUNT>` | Deposit stETH to mint ezETH (approve + deposit) |
| `renzo get-apr` | Get current restaking APR |
| `renzo balance [--address <ADDR>]` | Check ezETH and stETH balances |
| `renzo get-tvl` | Get protocol total value locked |

## Triggers
Activate when users want to deposit ETH into Renzo, restake ETH, get ezETH, check Renzo APR/balance/TVL, or perform liquid restaking operations. Also responds to Chinese phrases like 存款到Renzo, 再质押ETH, 获取ezETH.
