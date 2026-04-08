
# vertex-edge -- Skill Summary

## Overview
This skill provides comprehensive access to Vertex Edge, a cross-chain perpetual DEX primarily operating on Arbitrum. It enables querying market data, checking positions and balances, examining orderbook depth, retrieving current prices, and depositing USDC collateral. The plugin supports both read-only operations across multiple chains and write operations (deposits) on Arbitrum, making it essential for DeFi trading and portfolio management on Vertex Edge.

## Usage
Use commands like `vertex-edge get-markets` to list available markets, `vertex-edge get-positions` to check wallet balances, or `vertex-edge deposit` to add USDC collateral. All operations require specifying `--chain 42161` for Arbitrum, with additional parameters like wallet addresses or market symbols as needed.

## Commands
| Command | Description |
|---------|-------------|
| `get-markets` | List all Vertex Edge markets with prices and open interest |
| `get-positions` | View perp positions and spot balances for a wallet |
| `get-orderbook` | Query bid/ask depth for a specific market |
| `get-prices` | Get current mark and index prices for perp markets |
| `deposit` | Deposit USDC collateral (requires on-chain transactions) |

## Triggers
Activate this skill when users mention Vertex Edge trading activities, checking perpetual positions, querying market data, orderbook analysis, or depositing collateral. Trigger phrases include "vertex edge markets," "vertex perp positions," "vertex orderbook," "vertex deposit," and "vertex funding rate."
