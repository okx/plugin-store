
# kamino-liquidity -- Skill Summary

## Overview
This skill provides access to Kamino Liquidity KVault earn vaults on Solana, enabling users to deposit single tokens into auto-compounding vaults that automatically allocate liquidity to generate yield. Users receive vault shares representing their proportional stake and can track positions across multiple vaults while earning optimized returns through Kamino's automated strategies.

## Usage
Run commands with `kamino-liquidity <command>` on Solana mainnet (chain 501). Write operations require user confirmation and use `--dry-run` to preview transactions before execution.

## Commands
| Command | Description |
|---------|-------------|
| `vaults` | List all available KVault earn vaults with filtering options |
| `positions` | View current share balances across all user's vault positions |
| `deposit` | Deposit tokens into a vault to earn yield (requires confirmation) |
| `withdraw` | Redeem vault shares for underlying tokens (requires confirmation) |

## Triggers
Activate when users mention Kamino vault operations, liquidity farming, yield earning, or want to deposit/withdraw from KVaults on Solana. Also responds to Chinese terms like Kamino流动性, Kamino保险库, and 存入Kamino.
