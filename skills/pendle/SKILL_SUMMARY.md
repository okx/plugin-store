
# pendle -- Skill Summary

## Overview
The Pendle skill enables interaction with Pendle Finance's yield tokenization protocol, allowing users to split yield-bearing assets into Principal Tokens (PT) for fixed yields and Yield Tokens (YT) for floating yield exposure. It provides comprehensive trading capabilities including PT/YT buying and selling, liquidity provision and removal, token minting and redemption, plus portfolio monitoring across Ethereum, Arbitrum, BSC, and Base networks.

## Usage
Install the plugin and use natural language commands like "buy PT on Pendle" or "show my Pendle positions" to interact with yield markets. All write operations require user confirmation and support dry-run previews for safety.

## Commands
| Command | Description |
|---------|-------------|
| `list-markets` | Browse available Pendle markets across chains |
| `get-market` | Get detailed market information and APY history |
| `get-positions` | View current Pendle positions and holdings |
| `get-asset-price` | Get current prices for PT, YT, LP, or SY tokens |
| `buy-pt` | Purchase Principal Tokens for fixed yield exposure |
| `sell-pt` | Sell Principal Tokens back to underlying assets |
| `buy-yt` | Purchase Yield Tokens for floating yield speculation |
| `sell-yt` | Sell Yield Tokens back to underlying assets |
| `add-liquidity` | Provide single-token liquidity to Pendle AMM pools |
| `remove-liquidity` | Withdraw liquidity from Pendle pools |
| `mint-py` | Mint PT+YT pairs from underlying assets |
| `redeem-py` | Redeem PT+YT pairs back to underlying tokens |

## Triggers
Activate this skill when users want to trade fixed or floating yield positions, manage Pendle liquidity, or interact with yield tokenization features. Trigger phrases include "buy PT", "Pendle fixed yield", "add liquidity Pendle", "mint PT YT", or "Pendle positions".
