## Overview

Compound V2 is the original cToken-based money market on Ethereum mainnet. As of 2026, **all 6 major markets (cDAI / cUSDC / cUSDT / cETH / cWBTC2 / cCOMP) have governance-paused new supply** - V2 is in wind-down mode and Compound team's active development is on V3 (Comet). This plugin is positioned as an **exit tool**: redeem existing cToken positions, repay legacy debt, claim accrued COMP rewards. New supply / borrow attempts are caught at pre-flight and redirected to `compound-v3-plugin`.

## Prerequisites
- onchainos CLI installed and logged in
- ETH on Ethereum mainnet (>=0.005 ETH minimum to cover L1 gas; Compound V2 ops are gas-heavy)
- For exit flows: existing Compound V2 cToken supply OR borrow position. Run `quickstart` to detect.
- For new supply/borrow: install `compound-v3-plugin` instead - V2 is paused.

## Quick Start
1. Check your current state and get a guided next step: `compound-v2-plugin quickstart`
2. If you see `status: rpc_degraded` - public Ethereum RPC failed; wait a minute and retry: `compound-v2-plugin quickstart`
3. If you see `status: protocol_winddown` - you have no V2 positions and supply is paused. Install `compound-v3-plugin` for active flows: `npx skills add okx/plugin-store --skill compound-v3-plugin`
4. If you see `status: has_supply_can_redeem` - exit your supply position back to wallet: `compound-v2-plugin withdraw --token DAI --amount all --confirm`
5. If you see `status: has_debt_can_repay` - clear your debt cleanly (uint256.max sentinel, dust-free): `compound-v2-plugin repay --token USDC --all --confirm`
6. If you see `status: has_comp_accrued` - claim accumulated COMP rewards: `compound-v2-plugin claim-comp --confirm`
7. If you see `status: insufficient_gas` - top up at least 0.005 ETH on mainnet: `compound-v2-plugin markets`
8. To inspect markets / pause flags / APYs: `compound-v2-plugin markets` or per-position detail: `compound-v2-plugin positions`
