
# aave-v3-plugin -- Skill Summary

## Overview
This skill enables interaction with Aave V3, the leading decentralized lending protocol with over $43B TVL. Users can supply assets to earn yield, borrow against collateral, manage health factors, and monitor positions across Ethereum, Polygon, Arbitrum, and Base networks. The skill handles token approvals, constructs proper transaction calldata, and provides safety checks to prevent liquidation risks.

## Usage
Install the plugin binary and ensure your wallet is connected via `onchainos wallet login`. Use natural language phrases like "supply to aave", "borrow from aave", or "check my aave health factor" to trigger appropriate commands.

## Commands
| Command | Purpose |
|---------|---------|
| `aave-v3-plugin supply` | Deposit assets to earn interest |
| `aave-v3-plugin withdraw` | Redeem supplied assets |
| `aave-v3-plugin borrow` | Borrow assets against collateral |
| `aave-v3-plugin repay` | Repay borrowed debt |
| `aave-v3-plugin health-factor` | Check liquidation risk |
| `aave-v3-plugin positions` | View current portfolio |
| `aave-v3-plugin reserves` | List market interest rates |
| `aave-v3-plugin set-collateral` | Enable/disable collateral |
| `aave-v3-plugin set-emode` | Set efficiency mode |
| `aave-v3-plugin claim-rewards` | Claim accrued rewards |

## Triggers
Activate when users mention Aave operations like supplying, borrowing, checking health factors, or managing DeFi positions. Always run dry-run simulations first and confirm health factor safety before executing borrowing or collateral changes.
