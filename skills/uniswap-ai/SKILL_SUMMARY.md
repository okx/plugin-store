# uniswap-ai — Skill Summary

## Overview
The uniswap-ai plugin is an Nx monorepo of five focused Uniswap AI skills: `uniswap-trading` (swap integration and HTTP 402 pay-with-any-token), `uniswap-hooks` (V4 security foundations), `uniswap-cca` (Continuous Clearing Auction configurator and deployer), `uniswap-driver` (swap planner and liquidity planner deep links), and `uniswap-viem` (EVM integration via viem/wagmi). Together they provide end-to-end AI assistance for building on the Uniswap protocol across 12 EVM chains.

## Usage
Install the full suite with `npx skills add Uniswap/uniswap-ai` or install individual plugins via `claude plugin add @uniswap/uniswap-<name>`. Each sub-skill activates automatically when the agent detects relevant user intent such as swap integration, hook development, or CCA deployment.

## Commands
| Command | Description |
|---|---|
| `npx skills add Uniswap/uniswap-ai` | Install all 5 plugins |
| `claude plugin add @uniswap/uniswap-trading` | Install trading plugin only |
| `claude plugin add @uniswap/uniswap-hooks` | Install hooks security plugin only |
| `claude plugin add @uniswap/uniswap-cca` | Install CCA plugin only |
| `claude plugin add @uniswap/uniswap-driver` | Install swap/liquidity planner only |
| `claude plugin add @uniswap/uniswap-viem` | Install viem/wagmi integration only |

## Triggers
Activates when the user mentions Uniswap integration, swap development, V4 hook building, CCA auctions, liquidity position planning, or EVM TypeScript tooling with viem/wagmi. Each sub-skill has its own trigger set based on its specific domain.
