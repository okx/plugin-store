
# dolomite -- Skill Summary

## Overview
The dolomite skill enables interaction with Dolomite's isolated lending markets across EVM chains. Users can supply assets to earn yield, withdraw funds, view their positions, and simulate borrowing/repayment operations. All operations are executed through Dolomite's central DolomiteMargin contract using typed actions, with support for Arbitrum, Mantle, and Berachain networks.

## Usage
Run `dolomite --chain <id> <command>` to interact with Dolomite markets. Use `--dry-run` to preview transactions and `--confirm` to execute write operations after review.

## Commands
| Command | Description |
|---------|-------------|
| `markets` | List all available Dolomite lending markets with TVL and rates |
| `positions` | View current supply and borrow positions |
| `deposit` | Supply tokens to earn lending yield |
| `withdraw` | Withdraw supplied tokens from markets |
| `borrow` | Simulate borrowing tokens (dry-run only) |
| `repay` | Simulate debt repayment (dry-run only) |

## Triggers
Activate this skill when users want to interact with Dolomite lending markets, earn yield on crypto assets, manage lending positions, or explore borrowing opportunities. Use for DeFi lending operations across supported EVM chains.
