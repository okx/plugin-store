
# fenix -- Skill Summary

## Overview
The Fenix Finance plugin enables users to interact with the Fenix DEX on Blast network, providing functionality for token swaps, liquidity provision, price quotes, and pool exploration. Built on Algebra V4 MetaDEX, it supports concentrated liquidity market making (CLMM) with custom tick ranges and integrates with the ve33 tokenomics model for enhanced trading efficiency.

## Usage
Install the plugin via OKX plugin store and ensure your wallet is connected with `onchainos wallet login`. Use dry-run mode first for write operations, then execute after user confirmation.

## Commands
| Command | Description |
|---------|-------------|
| `get-quote` | Get swap quotes between token pairs |
| `swap` | Execute token swaps with slippage protection |
| `get-pools` | List available liquidity pools sorted by TVL |
| `add-liquidity` | Add concentrated liquidity positions with custom ranges |

## Triggers
Activate when users want to trade tokens on Blast network, get Fenix DEX quotes, manage liquidity positions, or explore available pools. Key phrases include "Fenix swap", "trade on Blast", "add liquidity Fenix", and "Fenix pools".
