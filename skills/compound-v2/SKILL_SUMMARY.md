
# compound-v2 -- Skill Summary

## Overview
This skill enables interaction with the deprecated Compound V2 lending protocol on Ethereum. While new deposits and borrows are frozen due to protocol deprecation, users can still view their positions, redeem existing cToken holdings to withdraw underlying assets, and claim accrued COMP rewards. The skill supports ETH, USDT, USDC, and DAI markets with both read-only operations and confirmed write transactions.

## Usage
Install the plugin and ensure onchainos CLI is available. Use `compound-v2 markets` and `compound-v2 positions` for read-only data, or execute write operations like `compound-v2 redeem` with the `--confirm` flag after previewing with `--dry-run`.

## Commands
| Command | Description |
|---------|-------------|
| `markets` | List cToken markets with APRs and exchange rates |
| `positions [--wallet addr]` | View your supply and borrow positions |
| `supply --asset TOKEN --amount N` | Supply assets (will fail due to frozen reserves) |
| `redeem --asset TOKEN --ctoken-amount N` | Redeem cTokens for underlying assets |
| `borrow --asset TOKEN --amount N` | Preview borrowing (dry-run only) |
| `repay --asset TOKEN --amount N` | Preview loan repayment (dry-run only) |
| `claim-comp` | Claim accrued COMP governance rewards |

## Triggers
Activate this skill when users want to check their Compound V2 positions, withdraw funds from existing cToken holdings, or claim COMP rewards. Also useful when users mention compound lending, cTokens, or need to exit deprecated V2 positions.
