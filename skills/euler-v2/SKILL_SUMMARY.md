
# euler-v2 -- Skill Summary

## Overview
This skill provides comprehensive interaction with Euler V2's modular lending protocol, enabling users to manage ERC-4626-compatible vault positions across multiple EVM chains. Users can supply assets to earn yield, withdraw funds, view market conditions, and simulate borrowing operations through the Ethereum Vault Connector (EVC) architecture.

## Usage
Install the plugin and ensure onchainos is available, then use commands like `euler-v2 --chain 8453 markets` to explore lending opportunities or `euler-v2 --chain 8453 supply --vault USDC --amount 10` to deposit assets. All write operations require `--confirm` flag for execution after previewing transaction details.

## Commands
| Command | Description |
|---------|-------------|
| `markets` | List available lending markets with TVL and rates |
| `positions` | View current supply and borrow positions |
| `supply` | Deposit assets into EVaults (requires --confirm) |
| `withdraw` | Withdraw assets from EVaults (requires --confirm) |
| `borrow` | Simulate borrow operations (dry-run only) |
| `repay` | Simulate repay operations (dry-run only) |

## Triggers
Activate this skill when users want to participate in DeFi lending, earn yield on crypto assets, or manage positions in Euler V2 protocol across Base, Ethereum, Arbitrum, Avalanche, or BSC networks. Use for both exploring lending markets and executing vault operations.
