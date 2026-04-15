
# aave-v3-plugin -- Skill Summary

## Overview
This plugin enables interaction with Aave V3, the leading decentralized lending protocol, allowing users to supply assets to earn yield, borrow against collateral, and manage positions across multiple chains. It provides comprehensive functionality for DeFi lending operations including health factor monitoring, collateral management, and reward claiming, with built-in safety mechanisms and automatic token handling.

## Usage
Connect your wallet with `onchainos wallet login`, then use commands like `aave-v3-plugin supply --asset USDC --amount 1000` to interact with Aave markets. Always simulate operations first (default behavior) before adding `--confirm` to execute transactions.

## Commands
| Command | Description |
|---------|-------------|
| `supply --asset <TOKEN> --amount <AMOUNT>` | Deposit assets to earn interest |
| `withdraw --asset <TOKEN> --amount <AMOUNT>` | Redeem supplied assets |
| `borrow --asset <ADDRESS> --amount <AMOUNT>` | Borrow against collateral |
| `repay --asset <TOKEN> --amount <AMOUNT>` | Repay borrowed debt |
| `health-factor` | Check liquidation risk status |
| `positions` | View current lending positions |
| `reserves` | List market rates and APYs |
| `set-collateral --asset <ADDRESS> --enable` | Manage collateral settings |
| `set-emode --category <ID>` | Enable efficiency mode |
| `claim-rewards` | Claim Aave incentive rewards |

## Triggers
Activate when users want to lend, borrow, or manage DeFi positions on Aave, especially with phrases like "supply to aave," "borrow from aave," "aave health factor," or "my aave positions." Always check health factor before borrowing operations to prevent liquidation risk.
