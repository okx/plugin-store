
# lido -- Skill Summary

## Overview
This skill enables interaction with the Lido liquid staking protocol on Ethereum mainnet, allowing users to stake ETH to receive stETH (liquid staking tokens), request withdrawals back to ETH, track staking rewards and APR, and manage the complete withdrawal lifecycle. All operations are secured through the onchainos CLI with user confirmation required for write transactions.

## Usage
Install the plugin via onchainos Plugin Store, ensure you're logged in with `onchainos wallet login` for write operations, then use commands like `lido stake --amount-eth 1.0` to begin staking.

## Commands
| Command | Description |
|---|---|
| `lido stake` | Stake ETH to receive stETH |
| `lido get-apy` | Get current stETH staking APR |
| `lido balance` | Check stETH balance |
| `lido request-withdrawal` | Request withdrawal of stETH for ETH |
| `lido get-withdrawals` | List pending and past withdrawal requests |
| `lido claim-withdrawal` | Claim finalized withdrawal(s) |

## Triggers
Activate this skill when users want to stake ETH for liquid staking rewards, need to withdraw staked ETH, or want to track their Lido staking positions and rewards. Use when users mention Lido, stETH, liquid staking, or ETH staking operations.
