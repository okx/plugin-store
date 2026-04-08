
# compound-v3 -- Skill Summary

## Overview
This plugin provides comprehensive access to Compound V3 (Comet) lending protocol across four major chains. Users can supply collateral assets, borrow USDC, repay debt, withdraw collateral, and claim COMP rewards. The plugin includes safety features like automatic debt repayment when supplying base assets, overflow protection for repayments, and pre-checks to prevent failed transactions.

## Usage
Install the plugin via OKX plugin store, ensure your wallet is connected with `onchainos wallet login`, then use commands like `compound-v3 supply`, `compound-v3 borrow`, or `compound-v3 get-position` to interact with lending markets.

## Commands
| Command | Purpose |
|---------|---------|
| `get-markets` | View market statistics (utilization, APRs, total supply/borrow) |
| `get-position` | Check account position (supply/borrow balances, collateral status) |
| `supply` | Supply collateral or base asset (auto-repays debt if applicable) |
| `borrow` | Borrow base asset against collateral |
| `repay` | Repay borrowed base asset (full or partial) |
| `withdraw` | Withdraw supplied collateral (requires zero debt) |
| `claim-rewards` | Claim COMP rewards from lending activities |

## Triggers
Activate this skill when users mention "compound supply", "compound borrow", "compound repay", "compound withdraw", "compound rewards", "compound position", or "compound market", or when they want to lend, borrow, or earn yield on major DeFi assets.
