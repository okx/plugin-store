
# lido -- Skill Summary

## Overview
This plugin enables interaction with the Lido liquid staking protocol on Ethereum mainnet, allowing users to stake ETH to receive stETH (a rebasing liquid staking token), request withdrawals back to ETH, and claim finalized withdrawals. All write operations require user confirmation before submission and route through the onchainos CLI for secure transaction handling.

## Usage
Ensure onchainos CLI ≥ 2.0.0 is installed and log in with `onchainos wallet login` for write operations. Use commands like `lido stake --amount-eth 1.0` to stake ETH or `lido balance` to check stETH holdings.

## Commands
| Command | Description |
|---|---|
| `lido stake` | Stake ETH to receive stETH (requires --confirm) |
| `lido get-apy` | Get current stETH staking APR |
| `lido balance` | Check stETH balance for an address |
| `lido request-withdrawal` | Request withdrawal of stETH for ETH (requires --confirm) |
| `lido get-withdrawals` | List pending and past withdrawal requests |
| `lido claim-withdrawal` | Claim finalized withdrawal(s) (requires --confirm) |

## Triggers
Activate when users want to stake ETH for liquid staking rewards, check stETH balances or APR, or manage withdrawal requests on the Lido protocol. Use for any Ethereum liquid staking operations involving stETH tokens.
