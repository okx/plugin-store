
# etherfi -- Skill Summary

## Overview
The etherfi plugin enables liquid restaking on Ethereum through the ether.fi protocol. Users can deposit ETH to receive eETH tokens, wrap them into yield-bearing weETH (ERC-4626) tokens that auto-compound staking and EigenLayer restaking rewards, and manage withdrawals through a two-step finalization process. The plugin provides real-time position tracking with APY data and USD valuations.

## Usage
Install the plugin and ensure onchainos CLI is configured with your wallet. Use `etherfi positions` to check balances and APY, then `etherfi stake --amount <ETH> --confirm` to deposit ETH for eETH tokens.

## Commands
- `etherfi positions [--owner <address>]` - View eETH/weETH balances, APY, and USD values
- `etherfi stake --amount <ETH> [--confirm]` - Deposit ETH to receive eETH tokens
- `etherfi wrap --amount <eETH> [--confirm]` - Wrap eETH into yield-bearing weETH
- `etherfi unwrap --amount <weETH> [--confirm]` - Unwrap weETH back to eETH
- `etherfi unstake --amount <eETH> [--confirm]` - Request ETH withdrawal (step 1)
- `etherfi unstake --claim --token-id <id> [--confirm]` - Claim ETH after finalization (step 2)

## Triggers
Activate when users want to stake ETH for liquid restaking rewards, wrap/unwrap between eETH and weETH tokens, withdraw staked ETH, or check their ether.fi positions and current APY rates. Also triggered by mentions of liquid restaking, EigenLayer rewards, or weETH tokens.
