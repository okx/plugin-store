
# gearbox-v3 -- Skill Summary

## Overview
This skill enables users to interact with the Gearbox V3 leverage protocol, allowing them to open Credit Accounts and amplify their DeFi positions through borrowing. Users can manage the complete lifecycle of leveraged positions including opening accounts, managing collateral, monitoring health factors, and closing positions across Arbitrum One and Ethereum Mainnet networks.

## Usage
Install the binary via the auto-injected setup commands, ensure your onchainos wallet is connected, then use commands like `gearbox-v3 open-account` to create leveraged positions or `gearbox-v3 get-account` to check existing accounts.

## Commands
| Command | Description |
|---------|-------------|
| `gearbox-v3 get-pools` | List all Credit Managers with debt limits and supported tokens |
| `gearbox-v3 get-account` | Check user's existing Credit Accounts and positions |
| `gearbox-v3 open-account` | Open new Credit Account with specified collateral and leverage |
| `gearbox-v3 add-collateral` | Add more collateral to existing Credit Account |
| `gearbox-v3 withdraw` | Withdraw collateral from Credit Account (partial or full) |
| `gearbox-v3 close-account` | Close Credit Account by repaying debt and withdrawing all collateral |

## Triggers
Activate this skill when users mention opening leveraged positions, borrowing against collateral, managing Gearbox Credit Accounts, or need exposure to leveraged DeFi strategies with phrases like "gearbox leverage", "credit account", or "leveraged yield".
