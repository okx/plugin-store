
# kamino-lend-plugin -- Skill Summary

## Overview
This plugin enables comprehensive interaction with Kamino Lend, Solana's leading borrowing and lending protocol. Users can view market data, manage lending positions, supply assets to earn yield, borrow against collateral, and repay loans. All operations include safety features like dry-run previews, health factor monitoring, and explicit user confirmation requirements for write operations.

## Usage
Install the plugin and ensure you're logged into onchainos on Solana mainnet (chain 501). Run commands with `--dry-run` first to preview transactions, then add `--confirm` to execute actual operations.

## Commands
| Command | Description |
|---------|-------------|
| `kamino-lend markets` | View lending markets with APYs and TVL |
| `kamino-lend positions` | Check your lending obligations and health factor |
| `kamino-lend supply --token <TOKEN> --amount <AMOUNT>` | Deposit assets to earn yield |
| `kamino-lend withdraw --token <TOKEN> --amount <AMOUNT>` | Withdraw supplied assets |
| `kamino-lend borrow --token <TOKEN> --amount <AMOUNT> --dry-run` | Borrow assets (preview) |
| `kamino-lend repay --token <TOKEN> --amount <AMOUNT> --dry-run` | Repay borrowed assets (preview) |

## Triggers
Activate when users want to interact with Kamino Lend for lending operations like checking market rates, managing positions, supplying/withdrawing assets, or borrowing/repaying loans. Use for DeFi yield farming and leveraged positions on Solana.
