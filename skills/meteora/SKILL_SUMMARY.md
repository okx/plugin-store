
# meteora -- Skill Summary

## Overview
The meteora plugin provides comprehensive access to Meteora's Dynamic Liquidity Market Maker (DLMM) protocol on Solana. It enables users to search liquidity pools, analyze trading opportunities, monitor their LP positions, and execute token swaps through an integrated DEX aggregator. The plugin combines read-only operations via direct API calls with secure write operations that require user confirmation.

## Usage
Use commands like `meteora get-pools` to search pools, `meteora get-swap-quote` for pricing, and `meteora swap --dry-run` to preview trades before execution. All swap operations require explicit user confirmation for security.

## Commands
| Command | Description |
|---------|-------------|
| `get-pools` | List and search Meteora DLMM pools with filtering options |
| `get-pool-detail` | Get detailed information for a specific pool |
| `get-swap-quote` | Get estimated swap quotes for token pairs |
| `get-user-positions` | View user's LP positions and unclaimed fees |
| `swap` | Execute token swaps with dry-run and confirmation support |

## Triggers
Activate when users mention Meteora swaps, DLMM pools, checking positions, or need to execute trades on the Meteora protocol. Also triggered by Chinese phrases like "Meteora换币" or "查询Meteora流动池".
