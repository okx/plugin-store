## Overview

Hyperliquid is a high-performance on-chain perpetuals DEX on its own L1, settling in USDC. This skill lets you trade perps & spot on the default DEX (BTC / ETH / SOL / 230+ crypto perps) AND on HIP-3 builder DEXs (xyz / flx / vntl / cash / km / etc. - RWAs like WTI Crude, BRENTOIL, GOLD, NVDA, TSLA, SP500, EUR/JPY). Bridge USDC from Arbitrum, trade with market/limit orders + TP/SL bracket, manage Health Factor + leverage, and move USDC between DEXs (each builder DEX has SEPARATE margin).

## Prerequisites
- onchainos CLI installed and logged in
- USDC on Arbitrum (chain 42161) to deposit into Hyperliquid (default DEX bridge)
- A small amount of ETH on Arbitrum for gas
- For HIP-3 (RWA / equity / commodity trading): additional `dex-transfer` to move USDC into the target builder DEX (each one has its own clearinghouse)

## Quick Start
1. Check your current state and get a guided next step: `hyperliquid-plugin quickstart`
2. If you see `status: no_funds` / `low_balance` - get your deposit address and top up USDC on Arbitrum: `hyperliquid-plugin address`
3. If you see `status: needs_deposit` - bridge Arbitrum USDC into Hyperliquid (arrives in 2-5 min): `hyperliquid-plugin deposit --amount 50 --confirm`
4. One-time: bind your signing address so orders can be signed: `hyperliquid-plugin register`
5. If you see `status: ready` - place your first perp order on the default DEX OR fund a HIP-3 builder DEX for RWAs: `hyperliquid-plugin order --coin BTC --side buy --size 0.001 --leverage 5 --confirm` OR `hyperliquid-plugin dex-transfer --to-dex xyz --amount 5 --confirm` (then trade `xyz:CL` / `xyz:NVDA` / etc.)
6. If you see `status: active` or `status: has_builder_dex_position` - review positions (pass `--dex xyz` for builder DEX positions) and attach stop-loss / take-profit: `hyperliquid-plugin positions --dex xyz` -> `hyperliquid-plugin tpsl --coin xyz:CL --sl-px 95 --tp-px 130 --confirm`
7. Close a position: `hyperliquid-plugin close --coin xyz:CL --confirm` (works for both default and builder DEX coins via the `dex:symbol` prefix)
8. Withdraw USDC: `hyperliquid-plugin dex-transfer --from-dex xyz --amount 5 --confirm` (back to default DEX) -> `hyperliquid-plugin withdraw --amount 50 --confirm` (default DEX -> Arbitrum)
