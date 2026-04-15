
# lido-plugin -- Skill Summary

## Overview
The lido-plugin enables seamless interaction with Lido's liquid staking protocol on Ethereum mainnet. Users can stake ETH to receive rebasing stETH tokens, request withdrawals back to ETH, monitor withdrawal queues, and claim finalized ETH withdrawals. All operations include transaction previews with user confirmation and integrate with the onchainos wallet system for secure signing.

## Usage
Install via the Plugin Store, ensure onchainos CLI is logged in for write operations, then use commands like `lido stake --amount-eth 1.0` or `lido get-apy` to interact with the protocol.

## Commands
| Command | Description |
|---------|-------------|
| `lido stake --amount-eth <amount>` | Stake ETH to receive stETH tokens |
| `lido get-apy` | Get current stETH staking APR and TVL data |
| `lido balance [--address <addr>]` | Check stETH balance for an address |
| `lido request-withdrawal --amount-eth <amount>` | Request withdrawal of stETH for ETH |
| `lido get-withdrawals [--address <addr>]` | List pending and past withdrawal requests |
| `lido claim-withdrawal --ids <id1,id2>` | Claim finalized withdrawal(s) |

## Triggers
Activate this skill when users want to stake ETH with Lido, check staking rewards/APY, manage liquid staking positions, or handle ETH withdrawals from the Lido protocol. Use for any Lido-related operations on Ethereum mainnet.
