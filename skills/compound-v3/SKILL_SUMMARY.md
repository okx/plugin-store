
# compound-v3 -- Skill Summary

## Overview
This plugin provides comprehensive Compound V3 (Comet) lending functionality, enabling users to supply collateral, borrow/repay USDC base assets, withdraw collateral, and claim COMP rewards across Ethereum, Base, Arbitrum, and Polygon networks. It includes intelligent features like automatic debt repayment when supplying base assets, overflow protection for repay operations, and mandatory dry-run previews for all write operations.

## Usage
Install the plugin via the OKX plugin store, ensure your onchainos wallet is connected, then use commands like `compound-v3 supply`, `compound-v3 borrow`, or `compound-v3 get-position`. All write operations require user confirmation after a dry-run preview.

## Commands
- `get-markets` - View market statistics (utilization, APRs, total supply/borrow)
- `get-position` - View account position (supply/borrow balances, collateralization status)
- `supply` - Supply collateral or base asset (auto-repays debt if supplying base asset)
- `borrow` - Borrow base asset (requires sufficient collateral)
- `repay` - Repay borrowed base asset (supports partial or full repayment)
- `withdraw` - Withdraw supplied collateral (requires zero outstanding debt)
- `claim-rewards` - Claim COMP rewards from CometRewards contract

## Triggers
Activate this skill when users mention compound lending operations, supplying collateral, borrowing USDC, repaying debt, withdrawing from compound, claiming COMP rewards, or checking compound positions. Trigger phrases include "compound supply," "compound borrow," "compound repay," "compound withdraw," "compound rewards," and "compound position."
