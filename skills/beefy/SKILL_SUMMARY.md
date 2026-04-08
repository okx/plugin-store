
# beefy -- Skill Summary

## Overview
The Beefy Finance skill enables interaction with Beefy's yield optimizer vaults across multiple EVM chains. It provides comprehensive vault management including browsing active vaults with APY data, depositing tokens for auto-compounding rewards, tracking mooToken positions, and withdrawing funds. The skill supports major chains like Base, BSC, Ethereum, Polygon, Arbitrum, and Optimism.

## Usage
Install the plugin and ensure onchainos CLI is available, then connect your wallet. Use read commands to explore vaults and APY rates, and write commands with `--confirm` flag to execute deposits and withdrawals.

## Commands
| Command | Description |
|---------|-------------|
| `beefy vaults --chain <id>` | List active vaults with APY and TVL |
| `beefy apy --chain <id>` | Show APY rates for vaults |
| `beefy positions --chain <id>` | View your mooToken balances |
| `beefy deposit --vault <name> --amount <value> --chain <id>` | Deposit tokens into vault |
| `beefy withdraw --vault <name> --chain <id>` | Redeem mooTokens |

## Triggers
Activate this skill when users want to explore yield farming opportunities, deposit into auto-compounding vaults, check their DeFi positions on Beefy Finance, or when they mention terms like "beefy vaults," "mooToken," "auto-compound," or yield optimization.
