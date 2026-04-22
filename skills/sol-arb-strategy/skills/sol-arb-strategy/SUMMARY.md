# sol-arb-strategy

## 1. Overview

An automated SOL arbitrage strategy that exploits price differences between Raydium and Orca DEXs on Solana.

Core operations:

- Monitor SOL/USDC prices on both Raydium and Orca
- Detect profitable spread opportunities
- Execute buy on cheaper DEX, sell on expensive DEX
- Report attribution for all trades

Tags: `solana` `arbitrage` `strategy` `raydium` `orca`

## 2. Prerequisites

- Supported chain: Solana
- Supported tokens: SOL, USDC
- Required plugins: raydium-plugin (^0.2.0), orca-plugin (^0.6.0)
- Funded Solana wallet with SOL and USDC

## 3. Quick Start

1. **Install dependencies**: Ensure raydium-plugin and orca-plugin are installed.
2. **Configure wallet**: Set up your Solana wallet via onchainos.
3. **Run strategy**: Execute `python3 scripts/arb.py --pair SOL/USDC --threshold 0.5` to start monitoring.
4. **Monitor**: The strategy logs all trades with attribution to `sol-arb-strategy`.
