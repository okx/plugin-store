# aerodrome-slipstream-plugin

## Overview

Aerodrome Slipstream is the concentrated liquidity (CL) AMM on Base, built on a Uniswap V3-style design with Velodrome's vote-escrowed tokenomics.

Core operations:

- Swap tokens via exactInputSingle with auto-routing across tick spacings
- Open and manage concentrated liquidity positions (NFT-based, via NFPM)
- Add and remove liquidity to existing positions
- Collect accumulated trading fees from LP positions
- Query pool state, spot prices, and swap quotes

Tags: `defi` `base` `amm` `liquidity` `swap` `aerodrome`

## Prerequisites

- No IP or region restrictions
- Supported chain: Base (8453)
- Supported tokens: any ERC-20 with a Slipstream CL pool on Base (WETH, USDC, cbBTC, AERO, and more)
- onchainos CLI installed and authenticated (`onchainos wallet login`)
- A funded wallet with the tokens you want to swap or deposit

## Quick Start

1. **Check available pools** — ask the agent to list pools for your token pair:
   "Show me WETH/USDC pools on Aerodrome Slipstream"

2. **Get a swap quote** — before executing, preview the expected output:
   "Quote swapping 0.01 WETH to USDC on Aerodrome"

3. **Swap tokens** — the agent will show a preview first, then ask for confirmation:
   "Swap 0.01 WETH to USDC on Aerodrome with 0.5% slippage"

4. **Open a liquidity position** — provide a tick range and token amounts:
   "Mint a WETH/USDC position with tick range -200000 to -197500, 0.01 WETH and 23 USDC"
   The agent will preview the position before executing.

5. **Collect fees** — after your position earns fees:
   "Collect fees from my Aerodrome position token ID 12345"

6. **Remove liquidity** — specify a percentage or remove all:
   "Remove 50% of liquidity from position 12345"
