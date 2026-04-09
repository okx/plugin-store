
# aave-v3 -- Skill Summary

## Overview
The aave-v3 skill enables users to interact with Aave V3, the leading decentralized lending protocol, across multiple chains including Ethereum, Polygon, Arbitrum, and Base. It provides comprehensive functionality for supplying assets to earn yield, borrowing against collateral, managing health factors to avoid liquidation, and monitoring positions in real-time.

## Usage
Install the plugin and connect your wallet with `onchainos wallet login`, then use natural language triggers like "supply to aave", "borrow from aave", or "check my aave health factor" to interact with the protocol. All write operations include dry-run simulation and require user confirmation before execution.

## Commands
| Command | Purpose |
|---------|---------|
| `aave-v3 supply` | Deposit assets to earn interest |
| `aave-v3 withdraw` | Redeem supplied assets |
| `aave-v3 borrow` | Borrow against collateral |
| `aave-v3 repay` | Repay borrowed debt |
| `aave-v3 health-factor` | Check liquidation risk |
| `aave-v3 positions` | View current positions |
| `aave-v3 reserves` | List market rates and APYs |
| `aave-v3 set-collateral` | Enable/disable asset as collateral |
| `aave-v3 set-emode` | Set efficiency mode for correlated assets |
| `aave-v3 claim-rewards` | Claim accrued protocol rewards |

## Triggers
Activate this skill when users want to lend, borrow, or manage positions on Aave V3, including phrases like "supply to aave", "borrow from aave", "aave health factor", "my aave positions", or when users need to check interest rates or manage collateral settings. Always prioritize safety by checking health factors before risky operations.
