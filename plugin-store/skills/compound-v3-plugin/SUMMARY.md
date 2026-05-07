## Overview

Compound V3 (Comet) is a single-asset lending protocol on Ethereum, Base, Arbitrum, and Polygon. This skill lets you supply the base asset to earn yield, supply collateral to borrow, repay and withdraw, check positions, and claim COMP rewards.

## Prerequisites
- onchainos CLI installed and logged in
- ETH for gas on the target chain (Ethereum / Base / Arbitrum / Polygon)
- USDC or WETH to supply as base asset, or a supported collateral asset (WETH, cbETH, ...) to borrow against

## Quick Start
1. Check your current state and get a guided next step: `compound-v3-plugin quickstart` (add `--chain <ID> --market <NAME>` to target a specific market; default is Base USDC)
2. If you see `status: new_user` — browse market rates and collateral assets, then supply the base asset: `compound-v3-plugin --chain 8453 --market usdc get-markets` → `compound-v3-plugin --chain 8453 --market usdc supply --asset <BASE_ASSET_ADDR> --amount 10 --confirm`
3. If you see `status: earning` — view your position and accrued interest, claim COMP rewards when available: `compound-v3-plugin --chain 8453 --market usdc get-position` / `compound-v3-plugin --chain 8453 --market usdc claim-rewards --confirm`
4. If you see `status: borrowed` — review position and health (pass `--collateral-asset <ADDR>` to see collateral), then repay when ready: `compound-v3-plugin --chain 8453 --market usdc get-position --collateral-asset <ADDR>` → `compound-v3-plugin --chain 8453 --market usdc repay --amount all --confirm`
5. To borrow from a fresh wallet: supply a collateral asset, then borrow the base asset: `compound-v3-plugin --chain 8453 --market usdc supply --asset <COLLATERAL_ADDR> --amount 0.005 --confirm` → `compound-v3-plugin --chain 8453 --market usdc borrow --amount 5 --confirm`
6. Exit a borrow: repay first, then withdraw collateral: `compound-v3-plugin --chain 8453 --market usdc repay --amount all --confirm` → `compound-v3-plugin --chain 8453 --market usdc withdraw --asset <COLLATERAL_ADDR> --amount 0.005 --confirm`
