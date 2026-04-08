
# eigencloud -- Skill Summary

## Overview
This skill enables interaction with EigenLayer's restaking protocol on Ethereum mainnet, allowing users to deposit liquid staking tokens (LSTs) into strategies, delegate to operators running Actively Validated Services, and manage withdrawals. It supports the complete restaking lifecycle from initial deposits through reward earning and eventual withdrawals, with comprehensive position tracking and strategy management.

## Usage
Install the eigencloud binary and ensure onchainos CLI is configured with your wallet. Use commands like `eigencloud strategies` to view available strategies, `eigencloud deposit` to restake LSTs, and `eigencloud delegate` to choose operators.

## Commands
| Command | Description |
|---------|-------------|
| `strategies` | List all EigenLayer strategies and their total shares |
| `positions` | View restaking positions and delegation status for a wallet |
| `deposit --token <TOKEN> --amount <AMOUNT>` | Deposit LST into strategy (requires --confirm to broadcast) |
| `delegate --operator <ADDRESS>` | Delegate to an operator (requires --confirm to broadcast) |
| `queue-withdraw --token <TOKEN> --shares <AMOUNT>` | Queue withdrawal with 7-day delay (requires --confirm to broadcast) |

## Triggers
An AI agent should activate this skill when users want to restake LST tokens on EigenLayer, delegate to operators for AVS rewards, check their restaking positions, or manage withdrawals from EigenLayer strategies.
