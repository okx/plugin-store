
# etherfi -- Skill Summary

## Overview
The etherfi plugin enables liquid restaking on Ethereum through the ether.fi protocol. Users can deposit ETH to receive eETH liquid staking tokens, wrap eETH into weETH (an ERC-4626 yield-bearing token) that auto-compounds staking plus EigenLayer restaking rewards, and manage their positions with real-time APY tracking. All write operations use a preview-then-confirm pattern for transaction safety.

## Usage
Run `etherfi positions` to check current balances and APY. For transactions, first run commands without `--confirm` to preview, then add `--confirm` to broadcast. Ensure onchainos CLI is installed and wallet is configured.

## Commands
- `etherfi positions [--owner <address>]` - View eETH/weETH balances and protocol APY
- `etherfi stake --amount <eth> [--confirm]` - Deposit ETH to receive eETH
- `etherfi wrap --amount <eeth> [--confirm]` - Wrap eETH into weETH (ERC-4626)
- `etherfi unwrap --amount <weeth> [--confirm]` - Unwrap weETH back to eETH
- `etherfi unstake --amount <eeth> [--confirm]` - Request withdrawal (step 1)
- `etherfi unstake --claim --token-id <id> [--confirm]` - Claim ETH after finalization (step 2)

## Triggers
Activate when users want to stake ETH on ether.fi, manage eETH/weETH positions, earn liquid staking plus EigenLayer restaking rewards, or check their ether.fi portfolio and APY. Also triggered by phrases like "liquid restaking", "wrap eETH", or "ether.fi positions".
