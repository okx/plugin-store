
# pendle-plugin -- Skill Summary

## Overview
The pendle-plugin provides comprehensive access to Pendle Finance's yield tokenization protocol, allowing users to split yield-bearing assets into Principal Tokens (PT) for fixed yield and Yield Tokens (YT) for variable yield exposure. It supports trading these tokens, providing liquidity to AMM pools, and managing positions across Ethereum, Arbitrum, BSC, and Base networks through integration with the Pendle API and on-chain smart contracts.

## Usage
Install the plugin via the auto-injected dependencies, ensure onchainos wallet is connected, then use commands like `pendle list-markets` to browse available pools or `pendle buy-pt` to purchase fixed-yield tokens. All write operations follow a preview-then-confirm flow for safety.

## Commands
| Command | Purpose |
|---------|---------|
| `list-markets` | Browse available Pendle markets across chains |
| `get-market` | Get detailed market information and APY history |
| `get-positions` | View current Pendle positions and balances |
| `get-asset-price` | Query prices for PT, YT, LP, or SY tokens |
| `buy-pt` | Purchase Principal Tokens for fixed yield exposure |
| `sell-pt` | Sell Principal Tokens back to underlying assets |
| `buy-yt` | Purchase Yield Tokens for leveraged yield speculation |
| `sell-yt` | Sell Yield Tokens back to underlying assets |
| `add-liquidity` | Provide single-token liquidity to Pendle AMM pools |
| `remove-liquidity` | Withdraw liquidity from Pendle pools |
| `mint-py` | Split underlying assets into PT+YT token pairs |
| `redeem-py` | Combine PT+YT tokens back to underlying assets |

## Triggers
Activate this skill when users mention yield trading, fixed income, Pendle markets, PT/YT tokens, or want to provide liquidity to yield-bearing pools. Also triggered by Chinese phrases like "购买PT", "出售YT", "Pendle固定收益", and "Pendle流动性".
