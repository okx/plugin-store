
# compound-v3 -- Skill Summary

## Overview
This skill enables interaction with Compound V3 (Comet) lending markets across Ethereum, Base, Arbitrum, and Polygon. It provides complete lending functionality including supplying collateral, borrowing base assets, repaying debt, withdrawing collateral, and claiming COMP rewards. All write operations require user confirmation and support dry-run previews for safety.

## Usage
Install the plugin via OKX plugin store, ensure your wallet is connected with `onchainos wallet login`, then use commands like `compound-v3 supply`, `compound-v3 borrow`, or `compound-v3 repay` with appropriate parameters.

## Commands
- `get-markets` - View market statistics (utilization, APRs, total supply/borrow)
- `get-position` - View account position (supply/borrow balances, collateralization)
- `supply` - Supply collateral or base asset (auto-repays debt if supplying base)
- `borrow` - Borrow base asset (requires sufficient collateral)
- `repay` - Repay borrowed base asset (partial or full repayment)
- `withdraw` - Withdraw supplied collateral (requires zero debt)
- `claim-rewards` - Claim COMP rewards from CometRewards contract

## Triggers
Activate when users mention compound lending operations, supplying/borrowing assets on Compound, checking compound positions or markets, claiming COMP rewards, or managing compound debt positions. Trigger phrases include "compound supply", "compound borrow", "compound repay", "compound withdraw", "compound rewards", "compound position", and "compound market".
