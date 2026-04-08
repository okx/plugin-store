
# solayer -- Skill Summary

## Overview
The Solayer skill enables interaction with the Solayer liquid restaking protocol on Solana. Users can stake SOL to receive liquid sSOL tokens that earn restaking rewards, check current rates and APY, monitor their positions, and manage their liquid staking operations through simple CLI commands.

## Usage
Install via `npx skills add okx/plugin-store --skill solayer` and use commands like `solayer rates` to check APY or `solayer stake --amount 0.1` to stake SOL. All write operations require `--confirm` flag for security.

## Commands
| Command | Description |
|---------|-------------|
| `solayer rates` | Get current sSOL/SOL exchange rate, APY, and TVL |
| `solayer positions` | Check your sSOL balance and SOL equivalent |
| `solayer stake --amount <SOL>` | Stake SOL to receive sSOL |
| `solayer unstake --amount <sSOL>` | Unstake sSOL (guides to Solayer UI) |

## Triggers
Activate when users want to stake SOL on Solayer, check sSOL rates/APY, monitor their liquid staking positions, or perform restaking operations. Common phrases include "stake SOL Solayer", "sSOL APY", "Solayer rates", and "check sSOL balance".
