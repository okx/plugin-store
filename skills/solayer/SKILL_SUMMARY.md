
# solayer -- Skill Summary

## Overview
This skill enables interaction with the Solayer liquid restaking protocol on Solana, allowing users to stake SOL tokens to receive sSOL (liquid staking tokens) and earn restaking rewards. It provides comprehensive functionality for checking staking rates, managing positions, and executing stake/unstake operations through secure on-chain transactions.

## Usage
Install the plugin and ensure onchainos CLI is available for wallet operations. Run commands without `--confirm` first to preview transactions, then add `--confirm` to execute on-chain operations.

## Commands
| Command | Description |
|---------|-------------|
| `solayer rates` | Get current sSOL/SOL exchange rate, APY, and TVL |
| `solayer positions` | Check your sSOL balance and SOL equivalent |
| `solayer stake --amount <SOL>` | Stake SOL to receive sSOL |
| `solayer unstake --amount <sSOL>` | Unstake sSOL (guides to Solayer UI) |

## Triggers
Activate when users want to stake SOL on Solayer, check sSOL rates/APY, view their liquid staking positions, or interact with Solana restaking protocols. Also triggered by phrases like "restake SOL", "sSOL balance", or "Solayer staking".
