## Overview

Supply, borrow and earn yield on **Euler v2** - a modular lending protocol with isolated-risk EVK (Euler Vault Kit) vaults. Each asset is its own independent vault; risk is segregated per-vault rather than pooled. Supports Ethereum, Base, and Arbitrum mainnet.

## Prerequisites
- onchainos CLI installed and a wallet added (`onchainos wallet add` or `onchainos wallet status`)
- Wallet has gas (ETH on Ethereum/Arbitrum, ETH on Base - bridged) on the chain you intend to use
- Wallet has assets to supply OR an existing supply position if you intend to borrow

## Quick Start
1. Check your current state and get a guided next step: `euler-v2-plugin quickstart --chain 1`
2. If you see `status: no_funds` - fund your wallet first, then re-run quickstart: `euler-v2-plugin quickstart --chain 1`
3. If you see `status: low_balance` - top up to >= $5 worth of asset, then re-run quickstart: `euler-v2-plugin quickstart --chain 1`
4. If you see `status: ready_to_supply` - browse vaults and pick one: `euler-v2-plugin list-vaults --chain 1 --limit 10`
5. If you see `status: active` - review your positions: `euler-v2-plugin positions --chain 1`
6. If you see `status: at_risk` - health factor < 1.5; repay or top up collateral: `euler-v2-plugin repay --vault <address> --all --chain 1`
7. If you see `status: liquidatable` - health factor < 1.0; repay immediately to avoid liquidation: `euler-v2-plugin repay --vault <address> --all --chain 1`
8. If you see `status: chain_invalid` - re-run with a supported chain: `euler-v2-plugin quickstart --chain 1`
