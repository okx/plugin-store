
# aave-v3 -- Skill Summary

## Overview
This skill provides comprehensive access to Aave V3, the leading decentralized lending protocol with over $43B TVL. Users can supply assets to earn yield, borrow against collateral, monitor health factors to avoid liquidation, view real-time market rates, and manage positions across Ethereum, Polygon, Arbitrum, and Base networks. The skill includes built-in safety features like health factor monitoring, dry-run simulations, and automatic liquidation risk warnings.

## Usage
Connect your wallet with `onchainos wallet login`, then use natural language commands like "supply 1000 USDC to Aave" or "check my Aave health factor". All transactions include confirmation prompts and dry-run simulations for safety.

## Commands
| Command | Purpose |
|---------|---------|
| `aave-v3 supply --asset <SYMBOL> --amount <AMOUNT>` | Deposit assets to earn interest |
| `aave-v3 withdraw --asset <SYMBOL> --amount <AMOUNT>` | Withdraw supplied assets |
| `aave-v3 borrow --asset <ADDRESS> --amount <AMOUNT>` | Borrow against collateral |
| `aave-v3 repay --asset <ADDRESS> --amount <AMOUNT>` | Repay borrowed debt |
| `aave-v3 health-factor` | Check liquidation risk status |
| `aave-v3 positions` | View current supply/borrow positions |
| `aave-v3 reserves` | List market interest rates and APYs |
| `aave-v3 set-collateral --asset <ADDRESS> --enable` | Enable/disable asset as collateral |
| `aave-v3 set-emode --category <ID>` | Set efficiency mode for higher leverage |
| `aave-v3 claim-rewards` | Claim accrued protocol rewards |

## Triggers
Activate this skill when users want to lend, borrow, or manage positions on Aave V3 using phrases like "supply to Aave," "borrow from Aave," "Aave health factor," "my Aave positions," or "Aave interest rates." Also triggers for collateral management and reward claiming activities.
