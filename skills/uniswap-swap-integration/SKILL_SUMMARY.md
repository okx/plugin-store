# uniswap-swap-integration — Skill Summary

## Overview
The uniswap-swap-integration skill guides developers and AI agents through integrating Uniswap token swaps into any application layer. It supports three integration methods: the Uniswap Trading API (best for frontends and backends), the Universal Router SDK (best for full transaction control), and direct smart contract calls via `execute()` on the Universal Router (best for on-chain DeFi composability). The skill includes strict input validation rules, mandatory user confirmation before any transaction, and complete Trading API reference documentation covering all routing types and order variants.

## Usage
Use this skill when building swap functionality into a React/Next.js frontend, a backend script or bot, or a Solidity smart contract. Authenticate Trading API requests with an `x-api-key` header from the Uniswap Developer Portal; install the `uniswap-viem` plugin for viem/wagmi setup before using this skill.

## Commands
| Endpoint | Method | Description |
|---|---|---|
| `POST /check_approval` | Trading API | Check and return approval transaction if needed |
| `POST /quote` | Trading API | Get executable quote with optimal routing |
| `POST /swap` | Trading API | Get signed transaction ready to submit |
| `SwapRouter.swapCallParameters()` | Universal Router SDK | Build calldata for Universal Router |

Base URL: `https://trade-api.gateway.uniswap.org/v1`

## Triggers
Activates when the user says "integrate swaps", "uniswap", "trading api", "add swap functionality", "build a swap frontend", "create a swap script", "smart contract swap integration", "use Universal Router", or mentions swapping tokens via Uniswap.
