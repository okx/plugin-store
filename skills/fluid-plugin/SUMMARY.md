# fluid-plugin

## Overview

Fluid Protocol is a smart-collateral lending platform by Instadapp, live on Ethereum and Arbitrum. Positions are ERC-721 NFTs managed through a single `operate()` entry point per vault.

Core operations:
- Supply collateral (ETH, wstETH, rETH, and other LSTs)
- Borrow stablecoins or ETH against your collateral
- Repay outstanding debt
- Withdraw collateral when your position is healthy

Tags: `defi` `lending` `ethereum` `arbitrum` `instadapp`

## Prerequisites

- No IP/region restrictions
- Supported chains: Ethereum (1), Arbitrum (42161)
- Supported tokens: ETH, wstETH, rETH, sfrxETH, cbETH, weETH, ezETH, mETH, USDC, USDT, DAI, WBTC, and more
- onchainos CLI installed and authenticated
- A wallet with collateral tokens funded on the target chain

## Quick Start

1. **Browse vaults**: `fluid vaults` to see all T1 pairs on Ethereum
2. **Preview supply**: `fluid supply --vault <addr> --amount 0.1` (safe — no broadcast)
3. **Execute supply**: Add `--confirm` to broadcast; note the `nft_id` in the response
4. **Borrow**: `fluid borrow --vault <addr> --nft-id <id> --amount 100 --confirm`
5. **Check positions**: `fluid positions` to see your open NFT positions
