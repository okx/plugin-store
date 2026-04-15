
# polymarket-plugin -- Skill Summary

## Overview

This skill enables AI agents to trade prediction markets on Polymarket, a decentralized platform on Polygon where users buy and sell outcome tokens representing real-world events. The plugin supports both direct wallet trading (EOA mode) and gasless proxy trading (POLY_PROXY mode), handling everything from market discovery to order execution, position management, and winning token redemption. It integrates with onchainos wallet for transaction signing and provides specialized features for short-term crypto prediction markets.

## Usage

Install the plugin and connect an onchainos wallet with a Polygon address, then check regional access with `polymarket-plugin check-access`. Choose between EOA mode (direct trading with gas costs) or proxy mode (one-time setup for gasless trading), fund with USDC.e, and start trading prediction markets.

## Commands

| Command | Description |
|---------|-------------|
| `check-access` | Verify region is not restricted |
| `list-markets` | Browse active prediction markets with filtering options |
| `list-5m` | List 5-minute crypto up/down markets |
| `get-market` | Get detailed market info and order book |
| `get-positions` | View current positions and P&L |
| `balance` | Show POL and USDC.e balances for both EOA and proxy wallets |
| `buy` | Buy YES/NO outcome shares |
| `sell` | Sell outcome shares |
| `cancel` | Cancel an open order |
| `redeem` | Redeem winning tokens after market resolution |
| `setup-proxy` | Deploy proxy wallet for gasless trading |
| `deposit` | Transfer USDC.e from EOA to proxy wallet |
| `switch-mode` | Switch between EOA and proxy trading modes |

## Triggers

Activate when users mention prediction market trading, Polymarket, betting on outcomes, buying YES/NO tokens, checking positions, or specific trigger phrases like "5-minute market", "breaking markets", or "trade on predictions". Also trigger for onboarding phrases like "new to polymarket" or "just installed polymarket".
