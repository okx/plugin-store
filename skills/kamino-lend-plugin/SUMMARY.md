## Overview

Kamino Lend is a leading lending protocol on Solana. This skill lets you browse lending markets and reserves with supply/borrow APYs, supply assets to earn yield, borrow against collateral, repay loans, withdraw, and monitor positions with health factors.

## Prerequisites
- onchainos CLI installed and logged in
- SOL for gas on Solana mainnet
- USDC (or another supported SPL token) to supply or use as collateral

## Quick Start
1. Check your state and get a guided next step: `kamino-lend-plugin quickstart`
2. If you see `status: no_funds` / `needs_gas` / `needs_funds` — fund the wallet address shown in the output (SOL for gas + USDC or another supported token to supply)
3. Browse available reserves with APYs: `kamino-lend-plugin reserves --min-apy 2`
4. View lending markets for deeper detail: `kamino-lend-plugin markets`
5. If `status: ready` — supply to earn yield (preview without `--confirm`, then re-run with it): `kamino-lend-plugin supply --asset USDC --amount 100 --confirm`
6. If `status: active` — review positions and health factor: `kamino-lend-plugin positions`
7. Borrow against your supplied collateral: `kamino-lend-plugin borrow --asset USDC --amount 50 --confirm`
8. Exit: `kamino-lend-plugin repay --asset USDC --amount all --confirm` or `kamino-lend-plugin withdraw --asset USDC --amount 100 --confirm`
