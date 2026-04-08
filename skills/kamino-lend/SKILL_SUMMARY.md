
# kamino-lend -- Skill Summary

## Overview
This skill provides comprehensive access to Kamino Lend, Solana's leading lending protocol, enabling users to supply assets for yield, borrow against collateral, and actively manage their lending positions. All operations include transaction previews and require explicit user confirmation before execution, with built-in safety checks for health factors and liquidation risks.

## Usage
Install the kamino-lend binary and ensure onchainos wallet access to Solana mainnet (chain 501). Run commands with dry-run flags first to preview transactions, then add --confirm to execute after user approval.

## Commands
| Command | Description | Example |
|---------|-------------|---------|
| `markets` | View lending markets with APYs and TVL | `kamino-lend markets` |
| `positions` | Check your lending obligations and health factor | `kamino-lend positions` |
| `supply` | Deposit assets to earn yield | `kamino-lend supply --token USDC --amount 0.01` |
| `withdraw` | Withdraw supplied assets | `kamino-lend withdraw --token USDC --amount 0.01` |
| `borrow` | Borrow assets against collateral | `kamino-lend borrow --token SOL --amount 0.001 --dry-run` |
| `repay` | Repay outstanding loans | `kamino-lend repay --token SOL --amount 0.001 --dry-run` |

## Triggers
Activate when users want to interact with Kamino lending markets, check lending positions, supply assets for yield, manage borrowing operations, or need current DeFi rates on Solana. Also triggered by mentions of Kamino Lend, Solana lending, or yield farming activities.
