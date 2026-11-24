## Overview

Puffer Finance is a liquid restaking protocol on Ethereum. Deposit ETH to mint pufETH (an ERC-4626 nLRT vault token) and earn restaking yield. Two exit paths: 1-step instant withdraw (1% fee, immediate WETH) or 2-step queued withdraw (~14 days, fee-free). All write operations require `--confirm`; signing routes through onchainos.

## Prerequisites
- onchainos CLI installed and logged in
- ETH on Ethereum mainnet (>=0.005 ETH minimum to cover gas + meaningful stake)
- For exit paths: existing pufETH position from a prior `stake`

## Quick Start
1. Check your current state and get a guided next step: `puffer-plugin quickstart`
2. If you see `status: rpc_degraded` - public Ethereum RPC failed; wait a minute and retry: `puffer-plugin quickstart`
3. If you see `status: no_funds` - wallet has neither pufETH nor stakeable ETH (>=0.005). Bridge ETH to mainnet first, then re-run quickstart.
4. If you see `status: ready_to_stake` - copy the recommended `next_command` to deposit ETH and mint pufETH: `puffer-plugin stake --amount 0.05 --confirm`
5. If you see `status: has_pufeth_earning` - view position then compare exit paths: `puffer-plugin withdraw-options --amount 0.01`
6. To exit the fast way (1-step, 1% fee, immediate WETH): `puffer-plugin instant-withdraw --amount 0.01 --confirm`
7. To exit the cheap way (2-step queued, no fee, ~14 days), first request a withdrawal: `puffer-plugin request-withdraw --amount 0.01 --confirm`
8. After ~14 days, check status by index then claim: `puffer-plugin claim-withdraw --index <N> --confirm`
