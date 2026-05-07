## Overview

Morpho is a permissionless lending protocol on Ethereum and Base. This skill lets you earn yield via MetaMorpho vaults, borrow against collateral on Morpho Blue isolated markets, repay, withdraw, monitor positions with health factors, and claim Merkl rewards.

## Prerequisites
- onchainos CLI installed and logged in
- ETH for gas on Ethereum mainnet (chain 1) or Base (chain 8453)
- USDC / WETH (or other supported asset) on the target chain to supply or use as collateral

## Quick Start
1. Check your state and get a guided next step: `morpho-plugin quickstart`
2. If you see `status: no_funds` / `needs_gas` / `needs_funds` — fund the wallet address shown in the output (ETH for gas, plus USDC or WETH to supply)
3. Browse available vaults and markets: `morpho-plugin vaults --asset USDC` or `morpho-plugin markets --asset USDC`
4. If `status: ready` — earn yield by supplying to a MetaMorpho vault: `morpho-plugin supply --vault <VAULT_ADDR> --asset USDC --amount 100 --confirm`
5. To leverage, supply collateral first then borrow: `morpho-plugin supply-collateral --market-id <HEX> --amount 1 --confirm` → `morpho-plugin borrow --market-id <HEX> --amount 1000 --confirm`
6. If `status: active` — review open positions and health factors: `morpho-plugin positions`
7. Exit: `morpho-plugin withdraw --vault <VAULT_ADDR> --asset USDC --all --confirm` or `morpho-plugin repay --market-id <HEX> --all --confirm`
8. Claim Merkl rewards when available: `morpho-plugin claim-rewards --confirm`
