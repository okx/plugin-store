
# zerolend -- Skill Summary

## Overview
This skill provides comprehensive interaction with ZeroLend, a verified Aave V3 fork operating on zkSync Era, Linea, and Blast networks. Users can supply assets to earn yield, borrow against collateral, manage health factors, and monitor positions through automated transaction handling and real-time data queries.

## Usage
Install the zerolend plugin and ensure your wallet is connected via `onchainos wallet login`. Use commands like `zerolend supply`, `zerolend borrow`, or `zerolend health-factor` with appropriate asset addresses and amounts.

## Commands
| Command | Description |
|---------|-------------|
| `zerolend supply --asset <ADDRESS> --amount <AMOUNT>` | Supply assets to earn interest |
| `zerolend withdraw --asset <SYMBOL> --amount <AMOUNT>` | Withdraw supplied assets |
| `zerolend borrow --asset <ADDRESS> --amount <AMOUNT>` | Borrow against collateral |
| `zerolend repay --asset <ADDRESS> --amount <AMOUNT>` | Repay borrowed debt |
| `zerolend health-factor` | Check position health and liquidation risk |
| `zerolend positions` | View current lending/borrowing positions |
| `zerolend reserves` | List market rates and APYs |
| `zerolend set-collateral --asset <ADDRESS> --enable <true/false>` | Enable/disable asset as collateral |
| `zerolend set-emode --category <ID>` | Set efficiency mode category |
| `zerolend claim-rewards` | Claim accrued protocol rewards |

Global flags: `--chain <CHAIN_ID>`, `--from <ADDRESS>`, `--dry-run`

## Triggers
Activate this skill when users mention ZeroLend operations like "supply to zerolend", "borrow on linea/blast", "zerolend health factor", or want to manage lending positions on zkSync Era, Linea, or Blast networks.
