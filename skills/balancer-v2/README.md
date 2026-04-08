# Balancer V2 Plugin

A Plugin Store skill for interacting with Balancer V2 — the multi-token AMM DEX on Ethereum and Arbitrum.

## Features

- **Swap tokens** via Balancer's single Vault entry point
- **Query pools** (top pools by liquidity, pool details)
- **Get quotes** using on-chain BalancerQueries contract
- **Add/remove liquidity** (joinPool, exitPool)
- **View LP positions** (BPT holdings)

## Supported Chains

- Arbitrum (42161) — primary
- Ethereum (1) — secondary

## Commands

| Command | Description |
|---------|-------------|
| `pools` | List top Balancer V2 pools |
| `pool-info` | Get pool details (tokens, balances, weights) |
| `quote` | Get swap quote via BalancerQueries |
| `positions` | View LP positions for connected wallet |
| `swap` | Execute token swap via Vault.swap() |
| `join` | Add liquidity via Vault.joinPool() |
| `exit` | Remove liquidity via Vault.exitPool() |

## Architecture

All on-chain writes route through the Balancer V2 **Vault** (`0xBA12222222228d8Ba445958a75a0704d566BF2C8`), which is the same address on all supported chains. Read operations use direct `eth_call` via public RPC endpoints, and pool discovery uses the Balancer Subgraph.

## License

MIT
