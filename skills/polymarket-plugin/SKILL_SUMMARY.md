
# polymarket-plugin -- Skill Summary

## Overview
This skill enables AI agents to interact with Polymarket, a prediction market platform on Polygon, allowing users to trade outcome tokens on real-world events. It supports both direct wallet trading (EOA mode) and gasless proxy wallet trading (POLY_PROXY mode), with automated credential management and EIP-712 signing through the onchainos wallet system.

## Usage
Connect your onchainos wallet, verify regional access with `polymarket-plugin check-access`, then fund your wallet with USDC.e on Polygon. Browse markets with `list-markets` and trade with `buy` or `sell` commands.

## Commands
| Command | Description |
|---------|-------------|
| `check-access` | Verify region is not restricted |
| `list-markets` | Browse active prediction markets with filtering options |
| `list-5m` | List 5-minute crypto up/down markets |
| `get-market` | Get detailed market information and order book |
| `get-positions` | View current positions and P&L |
| `balance` | Show POL and USDC.e balances for EOA and proxy wallets |
| `buy` | Purchase YES/NO outcome shares |
| `sell` | Sell existing outcome shares |
| `cancel` | Cancel open orders |
| `redeem` | Redeem winning tokens after market resolution |
| `setup-proxy` | Deploy proxy wallet for gasless trading |
| `deposit` | Transfer USDC.e from EOA to proxy wallet |
| `switch-mode` | Switch between EOA and proxy trading modes |

## Triggers
Activate when users want to trade prediction markets, check Polymarket positions, browse markets by topic (sports, elections, crypto), or search for specific events. Also triggers on setup phrases like "new to polymarket" or "just installed polymarket" for guided onboarding.
