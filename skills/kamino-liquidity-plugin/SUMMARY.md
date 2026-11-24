## Overview

Kamino Liquidity (KVaults) are automated yield-optimization vaults on Solana that accept single-token deposits (SOL / USDC / etc.) and earn yield via auto-compounding liquidity allocation. This skill lets you browse KVaults, deposit tokens, monitor positions, and withdraw shares.

## Prerequisites
- onchainos CLI installed and logged in
- SOL for gas on Solana mainnet
- USDC, SOL, or another supported SPL token to deposit

## Quick Start
1. Check your state and get a guided next step: `kamino-liquidity-plugin quickstart`
2. If you see `status: no_funds` / `needs_gas` / `needs_funds` — fund the wallet address shown in the output (SOL for gas + USDC or another supported token to deposit)
3. Browse available KVaults filtered by token: `kamino-liquidity-plugin vaults --token USDC`
4. Preview a deposit (no tx sent): `kamino-liquidity-plugin deposit --vault <VAULT_ADDR> --amount 10 --dry-run`
5. If `status: ready` — execute the deposit: `kamino-liquidity-plugin deposit --vault <VAULT_ADDR> --amount 10 --confirm`
6. If `status: active` — review your KVault positions with current share value: `kamino-liquidity-plugin positions`
7. Withdraw shares when ready: `kamino-liquidity-plugin withdraw --vault <VAULT_ADDR> --amount <SHARES> --confirm`
