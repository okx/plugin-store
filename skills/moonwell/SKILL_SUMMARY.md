
# moonwell -- Skill Summary

## Overview
The moonwell skill provides access to Moonwell's decentralized lending and borrowing protocol, a Compound V2 fork operating on Base, Optimism, and Moonbeam. Users can supply assets to earn interest and WELL rewards, redeem their mTokens, and safely preview borrowing operations. The protocol uses mTokens to represent user deposits and accrued interest, with built-in safety features including dry-run modes for risky operations.

## Usage
Install the moonwell plugin and ensure onchainos is configured with `onchainos wallet login`. Use commands with the `--dry-run` flag first to preview transactions, then add `--confirm` to execute write operations.

## Commands
| Command | Description |
|---------|-------------|
| `moonwell markets` | List all lending markets with APR and exchange rates |
| `moonwell positions` | View your supplied and borrowed balances |
| `moonwell supply --asset <SYMBOL> --amount <AMOUNT>` | Supply assets to earn interest |
| `moonwell redeem --asset <SYMBOL> --mtoken-amount <AMOUNT>` | Redeem mTokens for underlying assets |
| `moonwell borrow --asset <SYMBOL> --amount <AMOUNT> --dry-run` | Preview borrowing (dry-run only) |
| `moonwell repay --asset <SYMBOL> --amount <AMOUNT> --dry-run` | Preview repayment (dry-run only) |
| `moonwell claim-rewards` | Claim accrued WELL token rewards |

## Triggers
Activate this skill when users want to lend assets for yield, check lending market rates, manage existing positions on Moonwell, or claim WELL rewards. Also useful for users seeking to preview borrowing scenarios safely without liquidation risk.
