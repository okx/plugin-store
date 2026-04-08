
# aave-v3 -- Skill Summary

## Overview
The Aave V3 skill enables users to interact with the leading decentralized lending protocol with over $43B TVL. Users can supply assets to earn yield, borrow against collateral, monitor health factors to avoid liquidation, and manage positions across Ethereum, Polygon, Arbitrum, and Base networks. The skill integrates with onchainos wallet functionality and provides safety features including dry-run simulations and health factor warnings.

## Usage
Install the plugin, connect your wallet with `onchainos wallet login`, then use commands like `aave-v3 supply --asset USDC --amount 1000` to interact with the protocol. Always run with `--dry-run` first for borrowing and collateral operations.

## Commands
| Command | Description |
|---------|-------------|
| `supply` | Deposit assets to earn interest |
| `withdraw` | Redeem aTokens and withdraw assets |
| `borrow` | Borrow assets against collateral |
| `repay` | Repay borrowed debt |
| `health-factor` | Check account health and liquidation risk |
| `positions` | View current lending/borrowing positions |
| `reserves` | List market rates and APYs |
| `set-collateral` | Enable or disable assets as collateral |
| `set-emode` | Set efficiency mode for correlated assets |
| `claim-rewards` | Claim accrued lending/borrowing rewards |

## Triggers
Activate when users mention lending, borrowing, or DeFi operations with phrases like "supply to aave", "borrow from aave", "aave health factor", or "my aave positions". Also trigger for yield farming and collateral management requests.
