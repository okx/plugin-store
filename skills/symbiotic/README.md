# Symbiotic Plugin

Symbiotic restaking protocol integration for onchainos. Supports depositing collateral tokens into Symbiotic Vaults, checking positions, and managing restaking on Ethereum mainnet.

## Supported Operations

- `vaults` — List all Symbiotic vaults with TVL and APR
- `positions` — Check restaking positions across all vaults
- `rates` — View vault APR and reward rates
- `deposit` — Deposit collateral into a vault (wstETH, rETH, cbETH, etc.)
- `withdraw` — Request withdrawal from a vault (epoch-based, ~7 day delay)

## Supported Chains

- Ethereum Mainnet (chain ID: 1)

## Usage

```bash
symbiotic vaults
symbiotic rates --token wstETH
symbiotic positions
symbiotic deposit --token wstETH --amount 0.01 --dry-run
symbiotic withdraw --token wstETH --amount 0.01 --dry-run
```
