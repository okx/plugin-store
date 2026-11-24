# EigenCloud — Plugin Summary

**Version**: 0.1.1

## Overview

EigenLayer is a restaking protocol on Ethereum mainnet that lets holders of liquid staking tokens (LSTs) earn additional yield by securing Actively Validated Services (AVSs).

Core operations:
- List supported LST strategies (stETH, rETH, cbETH, and 8 more)
- View restaked positions and delegation status for any wallet
- Restake LSTs via approve + depositIntoStrategy (two-step flow)
- Delegate restaked shares to an AVS operator
- Undelegate and queue shares for withdrawal (7-day delay)

Tags: `defi` `ethereum` `restaking` `yield` `lrt`

## Prerequisites

- No IP/region restrictions
- Supported chain: Ethereum mainnet (chain ID 1)
- Supported tokens: stETH, rETH, cbETH, mETH, swETH, wBETH, sfrxETH, osETH, ETHx, ankrETH, EIGEN
- onchainos CLI installed and authenticated with an Ethereum mainnet wallet
- You must already hold an LST to restake (ETH itself is not supported)

## Quick Start

1. **Install**: `npx skills add okx/plugin-store --skill eigencloud-plugin`
2. **View supported tokens**: `eigencloud strategies`
3. **Check existing positions**: `eigencloud positions`
4. **Preview a stake**: `eigencloud stake --token stETH --amount 0.01`
5. **Execute**: `eigencloud stake --token stETH --amount 0.01 --confirm`
6. **Delegate to an operator**: `eigencloud delegate --operator 0xAddress --confirm`
