
# polymarket-plugin -- Skill Summary

## Overview
This plugin enables trading on Polymarket prediction markets directly from the command line. Users can buy and sell outcome tokens (YES/NO or categorical) for real-world events like elections, sports, and crypto prices. The plugin handles all authentication, order signing, and on-chain approvals automatically through integration with the onchainos wallet system. It supports both direct EOA trading and gasless proxy wallet mode for cost-effective trading.

## Usage
Install the plugin, connect your onchainos wallet to Polygon, verify regional access, then fund your wallet with USDC.e to start trading prediction markets.

## Commands
| Command | Description |
|---------|-------------|
| `check-access` | Verify region is not restricted |
| `list-markets [--keyword] [--breaking] [--category]` | Browse active prediction markets |
| `list-5m --coin <COIN>` | List 5-minute crypto up/down markets |
| `get-market --market-id <id>` | Get market details and order book |
| `get-positions` | View open positions |
| `balance` | Show POL and USDC.e balances |
| `buy --market-id <id> --outcome <yes/no> --amount <usdc>` | Buy outcome shares |
| `sell --market-id <id> --outcome <yes/no> --amount <shares>` | Sell outcome shares |
| `cancel --order-id <id>` | Cancel an open order |
| `redeem` | Redeem winning tokens after market resolves |
| `setup-proxy` | Deploy proxy wallet for gasless trading |
| `deposit --amount <N>` | Transfer USDC.e to proxy wallet |

## Triggers
Activate when users want to trade prediction markets, bet on real-world events, check Polymarket positions, or explore markets about politics, sports, crypto prices, or breaking news events.
