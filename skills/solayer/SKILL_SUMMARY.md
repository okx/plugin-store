
# solayer -- Skill Summary

## Overview
This skill enables interaction with the Solayer liquid restaking protocol on Solana, allowing users to stake SOL tokens to receive sSOL (liquid staking tokens) and earn restaking rewards. It provides comprehensive functionality for checking staking rates, monitoring positions, executing stake operations, and managing sSOL holdings through both read-only queries and confirmed on-chain transactions.

## Usage
Install the plugin and ensure onchainos CLI is available, then use commands like `solayer rates` to check current APY and `solayer stake --amount <SOL>` to stake tokens. All write operations require `--confirm` flag after previewing transaction details.

## Commands
| Command | Description |
|---------|-------------|
| `solayer rates` | Get current sSOL/SOL exchange rate, APY, and TVL |
| `solayer positions` | Check your sSOL balance and SOL equivalent |
| `solayer stake --amount <SOL>` | Stake SOL to receive sSOL |
| `solayer unstake --amount <sSOL>` | Unstake sSOL (guides to Solayer UI) |

## Triggers
Activate this skill when users mention staking SOL on Solayer, checking sSOL rates or balances, or wanting to interact with liquid restaking protocols. Use for phrases like "stake SOL Solayer", "get sSOL", "Solayer APY", or "check sSOL balance".
