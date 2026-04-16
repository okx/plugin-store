
# lido-plugin -- Skill Summary

## Overview
This skill enables interaction with the Lido liquid staking protocol on Ethereum mainnet, allowing users to stake ETH to receive stETH (a rebasing liquid staking token that grows daily), request withdrawals back to ETH, claim finalized withdrawals, and manage wrapped stETH (wstETH). All write operations require user confirmation and are executed through the secure onchainos CLI framework.

## Usage
Install the plugin through the onchainos Plugin Store, ensure you have onchainos CLI ≥ 2.0.0 installed, and log in with your wallet for write operations using `onchainos wallet login`.

## Commands
| Command | Description |
|---|---|
| `lido stake` | Stake ETH to receive stETH |
| `lido get-apy` | Get current stETH staking APR |
| `lido balance` | Check stETH balance |
| `lido request-withdrawal` | Request withdrawal of stETH for ETH |
| `lido get-withdrawals` | List pending and past withdrawal requests |
| `lido claim-withdrawal` | Claim finalized withdrawal(s) |
| `lido wrap` | Convert stETH to wstETH |
| `lido unwrap` | Convert wstETH to stETH |

## Triggers
Activate this skill when users want to stake ETH for liquid staking rewards, manage Lido staking positions, or perform operations with stETH/wstETH tokens. Use when users mention Lido protocol, liquid staking, stETH, or need to earn staking rewards while maintaining liquidity.
