
# aave-v3 -- Skill Summary

## Overview
This skill provides comprehensive access to Aave V3, the leading decentralized lending protocol, enabling users to supply assets for yield, borrow against collateral, monitor health factors, and manage positions across multiple chains (Ethereum, Polygon, Arbitrum, Base). All operations integrate with the onchainos wallet system and include safety checks like health factor monitoring and dry-run simulations to prevent liquidation risks.

## Usage
Connect your wallet via `onchainos wallet login`, then use natural language commands like "supply 1000 USDC to aave" or "check my aave health factor". All write operations are simulated first with `--dry-run` before requesting user confirmation.

## Commands
| Command | Description |
|---------|-------------|
| `aave-v3 supply --asset <TOKEN> --amount <AMOUNT>` | Deposit assets to earn interest |
| `aave-v3 withdraw --asset <TOKEN> --amount <AMOUNT>` | Withdraw supplied assets |
| `aave-v3 borrow --asset <ADDRESS> --amount <AMOUNT>` | Borrow against collateral |
| `aave-v3 repay --asset <ADDRESS> --amount <AMOUNT>` | Repay borrowed debt |
| `aave-v3 health-factor` | Check liquidation risk status |
| `aave-v3 positions` | View all current positions |
| `aave-v3 reserves` | List market rates and APYs |
| `aave-v3 set-collateral --asset <ADDRESS> --enable` | Enable/disable collateral |
| `aave-v3 set-emode --category <ID>` | Set efficiency mode |
| `aave-v3 claim-rewards` | Claim accrued rewards |

## Triggers
Activate when users mention lending, borrowing, earning yield, or specific Aave-related phrases like "supply to aave", "borrow from aave", "aave health factor", "my aave positions", or "aave interest rates". Also triggers on DeFi portfolio management requests involving collateral and liquidation risk monitoring.
