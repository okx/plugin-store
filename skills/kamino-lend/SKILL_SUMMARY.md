
# kamino-lend -- Skill Summary

## Overview
This skill provides complete access to Kamino Lend, Solana's leading lending protocol, enabling users to supply assets for yield, borrow against collateral, and manage lending positions. All operations include transaction previews and require explicit user confirmation before execution, with automatic integration to onchainos for seamless Solana blockchain interaction.

## Usage
Run commands like `kamino-lend markets` to view rates, `kamino-lend positions` to check your obligations, or `kamino-lend supply --token USDC --amount 0.01` to earn yield. All write operations require `--confirm` flag after preview.

## Commands
| Command | Description |
|---------|-------------|
| `kamino-lend markets` | View lending markets with APYs and TVL |
| `kamino-lend positions` | Check your lending obligations and health factor |
| `kamino-lend supply --token <TOKEN> --amount <AMOUNT>` | Supply assets to earn yield |
| `kamino-lend withdraw --token <TOKEN> --amount <AMOUNT>` | Withdraw supplied assets |
| `kamino-lend borrow --token <TOKEN> --amount <AMOUNT>` | Borrow assets (supports --dry-run) |
| `kamino-lend repay --token <TOKEN> --amount <AMOUNT>` | Repay borrowed assets (supports --dry-run) |

## Triggers
Activate when users want to interact with Kamino Lend for lending, borrowing, or yield farming on Solana, or when they ask about interest rates, lending positions, or DeFi yield opportunities on the Solana blockchain.
