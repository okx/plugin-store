## Overview

Aave V2 EXIT TOOL on Ethereum, Polygon, and Avalanche. As of 2026, all 57 V2 reserves across the 3 chains have `is_frozen=true` (governance-led wind-down for V3 migration) - new supply and borrow are rejected on-chain with `VL_RESERVE_FROZEN`. This plugin handles legacy V2 position management: redeem aTokens, repay debt cleanly via uint256.max sentinel, claim stkAAVE/WMATIC/WAVAX rewards, swap borrow rate mode. New supply/borrow attempts are caught at pre-flight and return a structured `RESERVE_FROZEN` error with a `redirect` field that points Agents to install and use `aave-v3-plugin`.

## Prerequisites
- onchainos CLI installed and logged in
- Native gas on the target chain (mainnet: >=0.005 ETH; Polygon: >=0.1 MATIC; Avalanche: >=0.05 AVAX)
- For exit flows: existing Aave V2 supply or borrow position. Run quickstart to detect.
- For new supply/borrow: install `aave-v3-plugin` instead - V2 is fully frozen.

## Quick Start
1. Check your current state and get a guided next step on a specific chain: `aave-v2-plugin quickstart` (defaults to Ethereum; pass `--chain POLYGON` or `--chain AVAX` for other deployments)
2. If you see `status: rpc_degraded` - public RPC failed; wait a minute and retry: `aave-v2-plugin quickstart`
3. If you see `status: protocol_winddown` - you have no V2 positions and all reserves are frozen. Install aave-v3-plugin via the next_command: `npx skills add okx/plugin-store --skill aave-v3-plugin --yes --global`
4. If you see `status: has_supply_can_redeem` - exit your supply position back to wallet: `aave-v2-plugin withdraw --token USDC --amount all --confirm`
5. If you see `status: has_active_borrow` - close debt cleanly (uint256.max sentinel, dust-free): `aave-v2-plugin repay --token USDT --all --rate-mode 2 --confirm`
6. If you see `status: has_rewards_accrued` - claim accrued stkAAVE/WMATIC/WAVAX rewards: `aave-v2-plugin claim-rewards --confirm`
7. If you see `status: unhealthy_position` - HF below safe threshold; immediately repay debt or add collateral. Run `aave-v2-plugin positions` for full breakdown.
8. If you see `status: insufficient_gas` - top up native gas on the target chain: `aave-v2-plugin markets --chain ETH`
