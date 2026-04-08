
# aave-v3 -- Skill Summary

## Overview
This skill enables users to interact with Aave V3, the leading decentralized lending protocol, allowing them to supply assets to earn yield, borrow against collateral, manage health factors, and monitor positions across Ethereum, Polygon, Arbitrum, and Base networks. The skill provides comprehensive safety features including health factor monitoring, dry-run simulations, and automatic liquidation risk warnings.

## Usage
Connect your wallet with `onchainos wallet login`, then use commands like `aave-v3 supply --asset USDC --amount 1000` to start earning interest. Always run `--dry-run` first for borrowing operations to check health factors and confirm safety.

## Commands
| Command | Description |
|---------|-------------|
| `aave-v3 supply --asset <ASSET> --amount <AMOUNT>` | Supply assets to earn interest |
| `aave-v3 withdraw --asset <ASSET> --amount <AMOUNT>` | Withdraw supplied assets |
| `aave-v3 borrow --asset <ADDRESS> --amount <AMOUNT>` | Borrow against collateral |
| `aave-v3 repay --asset <ADDRESS> --amount <AMOUNT>` | Repay borrowed debt |
| `aave-v3 health-factor` | Check liquidation risk |
| `aave-v3 positions` | View current positions |
| `aave-v3 reserves` | List market rates and APYs |
| `aave-v3 set-collateral --asset <ADDRESS> --enable` | Enable/disable collateral |
| `aave-v3 set-emode --category <ID>` | Set efficiency mode |
| `aave-v3 claim-rewards` | Claim accrued rewards |

## Triggers
Activate when users mention lending, borrowing, earning yield on DeFi, checking Aave positions, health factors, or collateral management. Also triggered by phrases like "supply to aave", "borrow from aave", "aave health factor", or "my aave positions".
