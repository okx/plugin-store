**Overview**

Compound V3 (Comet) is a single-asset lending protocol on Ethereum, Base, Arbitrum, and Polygon. This skill lets you supply the base asset to earn yield, supply collateral to borrow, repay and withdraw, check positions, and claim COMP rewards.

**Prerequisites**
- onchainos CLI installed and logged in
- ETH for gas on the target chain (Ethereum / Base / Arbitrum / Polygon)
- USDC or WETH to supply as base asset, or a supported collateral asset (WETH, cbETH, ...) to borrow against

**Quick Start**
1. Check your balance on the target chain: `onchainos wallet balance --chain 8453` (Base; use 1 / 42161 / 137 for Ethereum / Arbitrum / Polygon)
2. Browse market rates, utilization, and collateral assets: `compound-v3 --chain 8453 get-markets`
3. To earn yield, preview a supply of the base asset (no tx sent): `compound-v3 --chain 8453 --market usdc supply --asset <BASE_ASSET_ADDR> --amount 10`
4. Re-run with `--confirm` to execute: `compound-v3 --chain 8453 --market usdc --confirm supply --asset <BASE_ASSET_ADDR> --amount 10`
5. Check your position any time: `compound-v3 --chain 8453 --market usdc get-position`
6. To borrow, first supply a collateral asset, then borrow the base asset: `compound-v3 --chain 8453 --market usdc --confirm supply --asset <COLLATERAL_ADDR> --amount 0.005` → `compound-v3 --chain 8453 --market usdc --confirm borrow --amount 5`
7. Exit the borrow: `compound-v3 --chain 8453 --market usdc --confirm repay` then `compound-v3 --chain 8453 --market usdc --confirm withdraw --asset <COLLATERAL_ADDR> --amount 0.005`
8. Claim COMP rewards: `compound-v3 --chain 8453 --market usdc --confirm claim-rewards`
