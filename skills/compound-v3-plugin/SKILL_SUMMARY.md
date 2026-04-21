
# compound-v3-plugin -- Skill Summary

## Overview
This plugin provides complete access to Compound V3 (Comet) lending markets across Ethereum, Base, Arbitrum, and Polygon. Users can supply assets to earn yield, borrow against collateral, repay debt, withdraw funds, and claim COMP token rewards. The plugin emphasizes safety with preview-first execution and automatic collateralization validation before submitting transactions.

## Usage
Install via the OKX plugin store, connect your wallet with `onchainos wallet login`, then run `compound-v3 quickstart` for guided onboarding. All write operations use a preview-then-confirm workflow for safety.

## Commands
| Command | Purpose |
|---------|---------|
| `compound-v3 quickstart` | Check account status and get personalized next steps |
| `compound-v3 get-markets` | View current market rates and statistics |
| `compound-v3 get-position` | Check supply/borrow balances and collateral health |
| `compound-v3 supply --asset ADDRESS --amount X` | Supply collateral or base asset to earn yield |
| `compound-v3 borrow --amount X` | Borrow base asset against supplied collateral |
| `compound-v3 repay [--amount X]` | Repay borrowed funds (defaults to full repayment) |
| `compound-v3 withdraw --asset ADDRESS --amount X` | Withdraw supplied collateral (requires zero debt) |
| `compound-v3 claim-rewards` | Claim accrued COMP token rewards |

## Triggers
Activate when users mention Compound lending activities, DeFi yield farming, borrowing against crypto collateral, or need to check lending positions. Also trigger for phrases like "compound supply", "compound borrow", "compound rewards", or "lending rates".
