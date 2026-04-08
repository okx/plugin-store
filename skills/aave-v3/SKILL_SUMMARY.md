
# aave-v3 -- Skill Summary

## Overview
The aave-v3 skill enables users to interact with Aave V3, the leading decentralized lending protocol, supporting supply/withdraw operations to earn yield, borrowing against collateral, health factor monitoring, and position management across Ethereum, Polygon, Arbitrum, and Base networks. The skill automatically handles ERC-20 approvals, provides safety checks through dry-run simulations, and integrates with the onchainos CLI for secure transaction execution.

## Usage
Install the plugin and ensure your wallet is connected via `onchainos wallet login`. All operations support dry-run simulation before execution, and the skill automatically prompts for user confirmation before broadcasting transactions to the blockchain.

## Commands
| Command | Description |
|---------|-------------|
| `aave-v3 supply --asset <TOKEN> --amount <AMOUNT>` | Deposit assets to earn interest |
| `aave-v3 withdraw --asset <TOKEN> --amount <AMOUNT>` | Withdraw supplied assets |
| `aave-v3 borrow --asset <ADDRESS> --amount <AMOUNT>` | Borrow against collateral |
| `aave-v3 repay --asset <ADDRESS> --amount <AMOUNT>` | Repay borrowed debt |
| `aave-v3 health-factor` | Check liquidation risk |
| `aave-v3 positions` | View current positions |
| `aave-v3 reserves` | List market rates and APYs |
| `aave-v3 set-collateral --asset <ADDRESS> [--enable]` | Enable/disable collateral |
| `aave-v3 set-emode --category <ID>` | Set efficiency mode |
| `aave-v3 claim-rewards` | Claim accrued rewards |

## Triggers
Activate when users mention Aave-related operations like "supply to aave", "borrow from aave", "aave health factor", "my aave positions", "aave interest rates", "repay aave loan", "claim aave rewards", or want to manage DeFi lending and borrowing activities. The skill should also trigger for liquidation risk checks and collateral management tasks.
