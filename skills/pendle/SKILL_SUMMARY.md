
# pendle -- Skill Summary

## Overview
The Pendle plugin enables interaction with Pendle Finance's yield tokenization protocol, allowing users to split yield-bearing assets into Principal Tokens (PT) for fixed yield and Yield Tokens (YT) for floating yield exposure. Users can trade these tokens, provide liquidity to AMM pools, and manage their yield strategies across multiple chains including Ethereum, Arbitrum, BSC, and Base.

## Usage
Use natural language to express yield trading intentions like "buy PT for fixed yield" or "add liquidity to Pendle pool". All write operations include dry-run previews and require user confirmation before execution.

## Commands
| Command | Purpose |
|---------|---------|
| `list-markets` | Browse available Pendle markets and pools |
| `get-market` | Get detailed market information and APY history |
| `get-positions` | View current PT/YT/LP positions |
| `get-asset-price` | Check PT, YT, or LP token prices |
| `buy-pt` | Purchase Principal Tokens for fixed yield |
| `sell-pt` | Sell Principal Tokens |
| `buy-yt` | Purchase Yield Tokens for floating yield exposure |
| `sell-yt` | Sell Yield Tokens |
| `add-liquidity` | Provide single-token liquidity to pools |
| `remove-liquidity` | Withdraw liquidity from pools |
| `mint-py` | Mint PT+YT pairs from underlying assets |
| `redeem-py` | Redeem PT+YT pairs to underlying tokens |

## Triggers
Activate when users mention Pendle-specific yield strategies like "buy PT", "fixed yield", "yield tokenization", "Pendle liquidity", or want to split/combine PT and YT tokens. Use for yield trading and liquidity provision on Pendle Finance protocol.
