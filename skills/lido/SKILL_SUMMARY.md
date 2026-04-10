
# lido -- Skill Summary

## Overview
This skill enables interaction with the Lido liquid staking protocol on Ethereum mainnet, allowing users to stake ETH for stETH liquid staking tokens, manage withdrawal requests through Lido's withdrawal queue, and track staking rewards. It provides both read-only operations for checking balances and APR rates, as well as write operations for staking, requesting withdrawals, and claiming finalized ETH.

## Usage
Install the plugin via the onchainos Plugin Store, then use commands like `lido stake --amount-eth 1.0` to stake ETH or `lido balance` to check your stETH balance. All write operations require user confirmation before broadcasting transactions to the blockchain.

## Commands
| Command | Description |
|---------|-------------|
| `lido stake` | Stake ETH to receive stETH |
| `lido get-apy` | Get current stETH staking APR |
| `lido balance` | Check stETH balance |
| `lido request-withdrawal` | Request withdrawal of stETH for ETH |
| `lido get-withdrawals` | List pending and past withdrawal requests |
| `lido claim-withdrawal` | Claim finalized withdrawal(s) |

## Triggers
An AI agent should activate this skill when users want to stake ETH with Lido, check stETH balances or rewards, or manage withdrawals from liquid staking positions. It's also appropriate for queries about current staking APR rates on Ethereum.
