# uniswap-liquidity-planner — Skill Summary

## Overview
The uniswap-liquidity-planner skill helps agents and users design and create liquidity positions on Uniswap V2, V3, and V4. It covers pool discovery, fee tier selection, and for V3/V4 pools, concentrated liquidity range configuration. The skill produces Uniswap app deep links that open pre-populated position creation forms. It is bundled in the `uniswap-driver` plugin alongside the swap planner and supports all 12 chains in the Uniswap ecosystem.

## Usage
Install via `claude plugin add @uniswap/uniswap-driver`. Describe the desired LP position (e.g. "add liquidity to ETH/USDC 0.05% pool on Arbitrum, range 1800–2200") and the skill will guide through fee tier selection, range setup, and generate the appropriate deep link or transaction parameters.

## Commands
This is a planning and reference skill with no standalone CLI commands. It generates Uniswap app deep links in the format `https://app.uniswap.org/add/...` based on user-specified token pairs, fee tiers, ranges, and chains.

## Triggers
Activates when the user wants to add liquidity to Uniswap, create an LP position, configure a concentrated liquidity range, or generate a Uniswap pool deep link for liquidity provision.
