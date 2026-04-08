
# gmx-v1 -- Skill Summary

## Overview
The gmx-v1 plugin enables trading on GMX V1, a decentralized perpetuals and spot trading protocol on Arbitrum and Avalanche. It provides functionality for leveraged trading, token swaps, and liquidity provision through GLP tokens. The plugin supports both read operations (checking prices and positions) and write operations (executing trades and managing liquidity) with direct execution for most operations and minimal keeper fees for perpetual positions.

## Usage
Install with `npx onchainos plugin install gmx-v1` and ensure you're logged into onchainos with a funded wallet. All write operations require `--confirm` flag after previewing transaction details.

## Commands
| Command | Purpose |
|---------|---------|
| `get-prices` | Fetch current oracle prices for all GMX V1 tokens |
| `get-positions` | View open perpetual positions for a wallet |
| `swap` | Swap ERC-20 tokens via GMX V1 Router |
| `buy-glp` | Mint GLP tokens by depositing ERC-20 tokens |
| `sell-glp` | Redeem GLP tokens for ERC-20 tokens |
| `open-position` | Open leveraged long/short perpetual positions |
| `close-position` | Close perpetual positions (partial or full) |
| `approve-token` | Approve ERC-20 tokens for GMX contracts |

## Triggers
Activate this skill when users want to trade perpetuals with leverage, swap tokens on GMX V1, manage GLP liquidity positions, or check current token prices and open positions on the GMX V1 protocol.
