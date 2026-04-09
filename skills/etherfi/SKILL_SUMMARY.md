
# etherfi -- Skill Summary

## Overview
The etherfi skill enables liquid restaking on Ethereum through the ether.fi protocol. Users can deposit ETH to receive eETH liquid staking tokens, wrap eETH into weETH (an ERC-4626 yield-bearing token) to earn auto-compounding staking and EigenLayer restaking rewards, check their positions and balances, and view current APY rates. The skill uses direct JSON-RPC calls for read operations and integrates with onchainos CLI for secure transaction broadcasting with mandatory confirmation gates.

## Usage
Install the etherfi binary and ensure onchainos CLI is configured with your wallet. Run commands without `--confirm` to preview transactions, then add `--confirm` to broadcast them to Ethereum mainnet.

## Commands
- `etherfi positions [--owner ADDRESS]` - View eETH/weETH balances and protocol APY
- `etherfi stake --amount ETH [--confirm]` - Deposit ETH to receive eETH
- `etherfi wrap --amount EETH [--confirm]` - Wrap eETH into yield-bearing weETH
- `etherfi unwrap --amount WEETH [--confirm]` - Redeem weETH back to eETH

## Triggers
An AI agent should activate this skill when users want to stake ETH on ether.fi, wrap/unwrap between eETH and weETH tokens, check their liquid restaking positions, or inquire about ether.fi APY rates and protocol statistics.
