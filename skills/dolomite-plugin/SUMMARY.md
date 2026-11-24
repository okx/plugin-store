## Overview

Dolomite is a decentralized money market and margin protocol on Arbitrum. Supply assets to earn interest, open borrow positions against your collateral, repay debt, and withdraw - all via the DolomiteMargin core contract. Unlike Aave/Compound, Dolomite uses a unified `deposit/withdraw` action model and supports 1000+ assets with isolated borrow positions (each with up to 32 collaterals).

## Prerequisites
- onchainos CLI installed and logged in
- ETH on Arbitrum mainnet (>=0.0005 ETH minimum to cover gas)
- Supportable token (USDC / USDT / WETH / DAI / WBTC / ARB / USDC.e / LINK) for supply, OR existing supply position for borrow/withdraw/repay flows

## Quick Start
1. Check your current state and get a guided next step: `dolomite-plugin quickstart`
2. If you see `status: rpc_degraded` - public Arbitrum RPC failed; wait a minute and retry: `dolomite-plugin quickstart`
3. If you see `status: no_funds` - wallet has no ETH gas, no supply, no borrow. Top up at least 0.001 ETH on Arbitrum: `dolomite-plugin markets`
4. If you see `status: needs_token` - you have ETH but no supportable tokens. View available markets: `dolomite-plugin markets`
5. If you see `status: ready_to_supply` - copy the recommended `next_command` to start earning interest: `dolomite-plugin supply --token WETH --amount 1 --confirm`
6. If you see `status: has_supply_earning` - view your accruing position and per-market APY: `dolomite-plugin positions`
7. If you see `status: has_borrow_position` - close debt cleanly (zero dust via Dolomite native sentinel): `dolomite-plugin repay --token USDT --all --position-account-number 100 --confirm`
8. To open a leveraged isolated position OR exit any position: `dolomite-plugin borrow --token USDT --amount 0.5 --collateral-token USDC --collateral-amount 1 --confirm` (opens position 100 with 1 USDC collateral, borrows 0.5 USDT) - and `dolomite-plugin withdraw --token USDC --amount 50 --confirm` to exit (use `--from-account-number N` to recover collateral from an isolated position)

