
# compound-v3 -- Skill Summary

## Overview
The compound-v3 skill enables comprehensive interaction with Compound V3 (Comet) lending protocol across multiple chains. It provides complete lending functionality including supplying collateral, borrowing base assets, repaying debt, withdrawing collateral, and claiming COMP rewards. The skill includes built-in safety mechanisms, supports dry-run previews, and provides real-time market data and position tracking.

## Usage
Use compound-v3 commands with optional chain and market parameters (defaults to Base chain and USDC market). All write operations support --dry-run mode for previewing before execution and require user confirmation before submitting transactions.

## Commands
- `get-markets` - View market statistics including utilization, APRs, and total supply/borrow
- `get-position` - View account position with supply/borrow balances and collateralization status  
- `supply` - Supply collateral or base asset (auto-repays debt if supplying base asset)
- `borrow` - Borrow base asset against supplied collateral
- `repay` - Repay borrowed base asset (full or partial repayment)
- `withdraw` - Withdraw supplied collateral (requires zero outstanding debt)
- `claim-rewards` - Claim COMP rewards from CometRewards contract

## Triggers
Activate this skill when users mention Compound protocol interactions, lending/borrowing operations, or specific trigger phrases like "compound supply", "compound borrow", "compound repay", "compound withdraw", "compound rewards", "compound position", or "compound market".
