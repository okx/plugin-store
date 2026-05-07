## Overview

Deploy ERC-20 tokens on Base via Clanker's AI-native launchpad — each token is automatically paired with WETH on Uniswap V4 at launch, and the deployer earns LP fees from every trade.

## Prerequisites
- onchainos agentic wallet connected
- Some ETH on Base for deployment gas

## How it Works
1. **Check your wallet**: Get a personalised next step based on your ETH balance on the active chain (Base by default). `clanker-plugin quickstart`
   - If `status: no_funds` or `needs_funds` — bridge or send ETH to your wallet on the active chain
   - If `status: ready` — proceed below
2. **Browse recent launches**: See recently deployed tokens and their on-chain metadata. `clanker-plugin list-tokens --limit 10`
3. **Search by creator**: Find tokens launched by a specific wallet or username. `clanker-plugin search-tokens --query <wallet-or-username>`
4. **Get token details**: View supply, Uniswap V4 pool address, and accrued LP fees for a token. `clanker-plugin token-info --address <contract>`
5. **Deploy a token**: Launch your ERC-20 — it's immediately paired with WETH on Uniswap V4 and tradeable. `clanker-plugin deploy-token --name "My Token" --symbol MTK --image-url <url> --confirm`
6. **Claim LP rewards**: Withdraw accumulated WETH trading fees from your token's pool to your wallet — also supported on Arbitrum One. `clanker-plugin claim-rewards --token-address <contract> --confirm`
