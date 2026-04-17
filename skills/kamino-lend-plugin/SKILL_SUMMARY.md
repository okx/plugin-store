
# kamino-lend-plugin -- Skill Summary

## Overview
This skill provides comprehensive access to Kamino Lend, the leading borrowing and lending protocol on Solana, enabling users to supply assets to earn yield, borrow against collateral, manage existing positions, and monitor lending markets with real-time APY data. All operations support dry-run previews and require explicit user confirmation before executing on-chain transactions.

## Usage
Install the plugin and ensure you're logged into onchainos with a Solana wallet. Run `kamino-lend quickstart` to check your wallet status and get started, then use commands like `kamino-lend reserves` to view available assets or `kamino-lend positions` to check your current lending obligations.

## Commands
| Command | Description |
|---------|-------------|
| `quickstart` | Check wallet status and get onboarding guidance |
| `reserves` | List all available lending assets with APYs |
| `markets` | View lending markets and interest rates |
| `positions` | View your current lending positions and health factor |
| `supply` | Supply assets to earn yield |
| `withdraw` | Withdraw supplied assets |
| `borrow` | Borrow assets against collateral (dry-run supported) |
| `repay` | Repay outstanding loans (dry-run supported) |

## Triggers
An AI agent should activate this skill when users want to interact with Kamino Lend for lending activities such as supplying assets to earn yield, borrowing against collateral, checking their lending positions, or viewing current market rates on Solana.
