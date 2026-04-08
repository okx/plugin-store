# Instadapp Lite Vaults Plugin

Instadapp Lite vault integration for the OKX Plugin Store. Supports depositing ETH into iETH v1 vault and stETH into iETHv2 vault, querying positions, and viewing yield rates.

## Vaults

| Vault | Symbol | Address | Deposit Token |
|-------|--------|---------|---------------|
| Instadapp Lite ETH | iETH | `0xc383a3833A87009fD9597F8184979AF5eDFad019` | ETH (native) |
| Instadapp Lite ETH v2 | iETHv2 | `0xa0d3707c569ff8c87fa923d3823ec5d81c98be78` | stETH (ERC-4626) |

## Commands

```bash
instadapp vaults                              # List vaults
instadapp rates                               # Show exchange price / yield
instadapp positions                           # Show your holdings
instadapp deposit --vault v1 --amount 0.0001  # Deposit 0.0001 ETH into iETH v1
instadapp withdraw --vault v1                 # Withdraw all iETH shares
instadapp deposit --vault v1 --amount 0.0001 --dry-run  # Simulate deposit
```

## Chain Support

- Ethereum (chain ID 1) — Instadapp Lite vaults are Ethereum mainnet only

## Build

```bash
cargo build --release
./target/release/instadapp vaults
```

## Architecture

- Read operations use direct `eth_call` via `https://ethereum.publicnode.com`
- Write operations use `onchainos wallet contract-call` CLI
- iETH v1: single `supplyEth(address)` transaction with ETH value
- iETHv2: 2-step flow (stETH approve + ERC-4626 deposit)
