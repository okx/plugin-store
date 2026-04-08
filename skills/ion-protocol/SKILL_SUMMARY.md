
# ion-protocol -- Skill Summary

## Overview
Ion Protocol is a CDP (Collateralized Debt Position) lending system that enables users to deposit liquid restaking tokens (LRT) and liquid staking tokens (LST) as collateral to borrow wstETH or WETH, while also allowing users to supply these loan tokens to earn yield. The plugin supports 4 active pools on Ethereum Mainnet with real-time rate monitoring and complete position management capabilities.

## Usage
Install the plugin via OKX plugin store, ensure wallet is connected with `onchainos wallet status`, then use commands like `ion-protocol get-pools` to view rates or `ion-protocol borrow` to open leveraged positions. All write operations require explicit confirmation before broadcasting transactions.

## Commands
- `get-pools` - List all 4 pools with current borrow APY and TVL
- `get-position` - Show vault position for a wallet address  
- `lend` - Supply wstETH or WETH to earn interest
- `withdraw-lend` - Withdraw previously lent tokens
- `deposit-collateral` - Deposit LRT collateral without borrowing
- `borrow` - Full flow: deposit collateral and borrow loan tokens
- `repay` - Repay borrowed debt with optional collateral withdrawal

## Triggers
Activate when users mention borrowing against liquid restaking tokens (rsETH, rswETH, ezETH, weETH), lending wstETH for yield, Ion Protocol specifically, or need CDP-style lending on Ethereum. Also trigger for queries about LRT collateral rates or leveraged staking strategies.
