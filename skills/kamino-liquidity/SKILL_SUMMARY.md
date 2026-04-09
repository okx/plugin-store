
# kamino-liquidity -- Skill Summary

## Overview
This skill provides comprehensive access to Kamino Liquidity's KVault earn vaults on Solana, enabling users to deposit single tokens into auto-compounding yield strategies. The plugin handles vault discovery, position tracking, and secure transaction execution through the Kamino API and onchainos wallet integration, with all write operations requiring explicit user confirmation before broadcasting to the blockchain.

## Usage
Install via `npx skills add okx/plugin-store --skill kamino-liquidity` and ensure onchainos CLI is available. Use commands with `--dry-run` first to preview transactions, then add `--confirm` to execute after user approval.

## Commands
| Command | Description |
|---------|-------------|
| `vaults` | List all available KVault earn vaults with filtering options |
| `positions` | View current share balances across all user's vault positions |
| `deposit` | Deposit tokens into a specified vault to earn yield |
| `withdraw` | Redeem vault shares for underlying tokens |

## Triggers
Activate when users mention Kamino vault operations, liquidity management, yield farming on Solana, or need to interact with KVault earn strategies. Also triggers on phrases like "Kamino liquidity," "deposit to Kamino," "Kamino earn," or "KVault."
