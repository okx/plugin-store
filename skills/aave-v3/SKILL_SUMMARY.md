
# aave-v3 -- Skill Summary

## Overview
This skill provides comprehensive access to Aave V3, the leading decentralized lending protocol with over $43B TVL. Users can supply assets to earn yield, borrow against collateral, monitor health factors, manage collateral settings, and claim rewards across Ethereum, Polygon, Arbitrum, and Base networks. All operations include safety checks and dry-run simulations to prevent liquidations.

## Usage
Connect your wallet with `onchainos wallet login`, then use natural language commands like "supply 1000 USDC to aave" or "check my aave health factor". All borrow and collateral operations automatically simulate first with `--dry-run` before requesting user confirmation.

## Commands
| Command | Purpose |
|---------|---------|
| `aave-v3 supply --asset <SYMBOL> --amount <AMOUNT>` | Deposit assets to earn interest |
| `aave-v3 withdraw --asset <SYMBOL> --amount <AMOUNT>` | Withdraw supplied assets |
| `aave-v3 borrow --asset <ADDRESS> --amount <AMOUNT>` | Borrow against collateral |
| `aave-v3 repay --asset <ADDRESS> --amount <AMOUNT>` | Repay borrowed debt |
| `aave-v3 health-factor` | Check liquidation risk status |
| `aave-v3 positions` | View all positions and balances |
| `aave-v3 reserves` | List current interest rates |
| `aave-v3 set-collateral --asset <ADDRESS> --enable` | Enable/disable collateral |
| `aave-v3 set-emode --category <ID>` | Set efficiency mode |
| `aave-v3 claim-rewards` | Claim accrued rewards |

## Triggers
Activate when users mention supplying, depositing, lending, borrowing, repaying loans, checking health factors, managing Aave positions, interest rates, collateral settings, or claiming rewards on Aave protocol.
