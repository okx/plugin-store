
# loopscale -- Skill Summary

## Overview
Loopscale enables lending and borrowing on Solana through an order-book credit protocol where lenders post fixed-rate offers and borrowers can fill them using any tokenized collateral. The plugin provides complete vault management, position tracking, and automated transaction handling for both lending and borrowing operations.

## Usage
Connect your Solana wallet via `onchainos wallet login`, then use commands like `loopscale lend` to deposit tokens for yield or `loopscale borrow` to take loans against collateral. Always run with `--dry-run` first to preview operations before execution.

## Commands
| Command | Purpose |
|---------|---------|
| `get-vaults` | List available lending vaults with APY and TVL data |
| `get-position` | View your active vault deposits and outstanding loans |
| `lend` | Deposit tokens into lending vaults to earn yield |
| `withdraw` | Withdraw tokens from lending vaults |
| `borrow` | Borrow tokens against collateral (two-step process) |
| `repay` | Repay outstanding loans partially or in full |

## Triggers
Activate when users want to lend tokens for yield on Solana, borrow against collateral with fixed rates, or manage existing Loopscale positions. Also trigger for queries about Solana lending protocols or order-book credit markets.
