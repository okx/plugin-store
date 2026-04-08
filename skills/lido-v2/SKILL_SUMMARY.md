
# lido-v2 -- Skill Summary

## Overview
This plugin enables comprehensive Lido liquid staking operations, allowing users to stake ETH for stETH, manage wstETH wrapping/unwrapping across multiple chains, and handle the complete ETH withdrawal process. It integrates with Lido's smart contracts on Ethereum mainnet and supports wstETH operations on Layer 2 networks including Arbitrum, Base, and Optimism.

## Usage
Install the plugin via OKX plugin store, ensure your wallet is connected with `onchainos wallet login`, then use commands like `lido stake --amount <wei>` to begin staking or `lido get-position` to check balances.

## Commands
- `lido get-apr` - Query current stETH staking APR
- `lido get-position` - Check stETH/wstETH balances across chains
- `lido get-withdrawal-status --request-ids <ids>` - Check withdrawal request status
- `lido stake --amount <wei>` - Stake ETH to receive stETH
- `lido wrap --amount <wei>` - Convert stETH to wstETH
- `lido unwrap --amount <wei>` - Convert wstETH back to stETH
- `lido request-withdrawal --amount <wei>` - Request ETH withdrawal from stETH
- `lido claim-withdrawal --request-ids <ids>` - Claim finalized withdrawals

## Triggers
Activate when users want to stake ETH with Lido, manage stETH/wstETH positions, or handle ETH withdrawals from liquid staking. Common phrases include "stake ETH lido", "lido staking", "wrap stETH", "unwrap wstETH", "request lido withdrawal", or "claim lido ETH".
