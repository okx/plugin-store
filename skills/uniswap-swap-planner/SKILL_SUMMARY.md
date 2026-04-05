# uniswap-swap-planner — Skill Summary

## Overview
The uniswap-swap-planner skill assists agents and users in planning token swap operations on Uniswap and generating shareable deep links that open directly in the Uniswap app pre-populated with swap parameters. It handles token discovery (resolving symbols to contract addresses), chain selection across 12 supported networks, and routing recommendations. It is part of the `uniswap-driver` plugin alongside the liquidity planner, and is designed to complement `uniswap-swap-integration` for users who need a planning step before programmatic execution.

## Usage
Install via `claude plugin add @uniswap/uniswap-driver`. Ask the agent to plan a swap (e.g. "swap 1 ETH for USDC on Base") and it will produce a deep link to the Uniswap interface and optionally guide through a full integration flow.

## Commands
This is a reference and planning skill with no standalone CLI commands. It generates Uniswap deep links in the format `https://app.uniswap.org/swap?...` based on user-specified token pairs, amounts, and chains.

## Triggers
Activates when the user wants to plan a Uniswap swap, look up token addresses, generate a Uniswap app deep link, or research token pairs across supported chains.
