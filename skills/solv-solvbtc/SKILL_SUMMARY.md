
# solv-solvbtc -- Skill Summary

## Overview
This skill enables interaction with Solv Protocol's SolvBTC system, allowing users to mint liquid BTC tokens by depositing WBTC, wrap them into yield-bearing xSolvBTC, and manage redemptions. The protocol operates on both Arbitrum and Ethereum, with SolvBTC maintaining 1:1 BTC backing and xSolvBTC generating yield through basis trading and staking strategies.

## Usage
Use trigger phrases like "mint SolvBTC", "wrap into xSolvBTC", "my SolvBTC balance", or "SolvBTC price" to activate the plugin. All transaction commands require explicit user confirmation before execution.

## Commands
| Command | Description |
|---------|-------------|
| `get-nav` | Fetch current SolvBTC/xSolvBTC prices and protocol TVL |
| `get-balance` | Query SolvBTC and xSolvBTC balances on specified chain |
| `mint` | Deposit WBTC to receive SolvBTC tokens |
| `redeem` | Submit non-instant withdrawal request to get WBTC back |
| `cancel-redeem` | Cancel pending redemption request |
| `wrap` | Wrap SolvBTC into yield-bearing xSolvBTC (Ethereum only) |
| `unwrap` | Unwrap xSolvBTC back to SolvBTC (Ethereum only) |

## Triggers
Activate when users want to interact with liquid BTC protocols, earn yield on Bitcoin holdings, or perform operations involving SolvBTC, xSolvBTC, or WBTC deposits/withdrawals. This skill is specifically for Solv Protocol operations on Arbitrum and Ethereum networks.
