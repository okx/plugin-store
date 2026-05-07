## Overview

Hyperliquid is a high-performance on-chain perpetuals DEX on its own L1, settling in USDC (perps) and USDH (HIP-4 outcomes). This skill lets you trade perps & spot on the default DEX (BTC / ETH / SOL / 230+ crypto perps), HIP-3 builder DEXs (xyz / flx / vntl / cash / km / etc. - RWAs like WTI Crude, GOLD, NVDA, TSLA, SP500, EUR/JPY, each with its OWN clearinghouse), AND HIP-4 outcome markets (binary YES/NO prediction contracts on real-world events, fully collateralized in USDH, no leverage / no liquidation, automatic settlement).

## Prerequisites
- onchainos CLI installed and logged in
- USDC on Arbitrum (chain 42161) to deposit into Hyperliquid (default DEX bridge)
- A small amount of ETH on Arbitrum for gas
- For HIP-3 (RWA / equity / commodity trading): additional `dex-transfer` to move USDC into the target builder DEX (each one has its own clearinghouse)
- For HIP-4 (outcome / prediction markets): `usdh-fund` to swap USDC for USDH on the spot pair (HL spot orders enforce a $10 minimum, including outcome orders)

## Quick Start
1. Check your current state and get a guided next step: `hyperliquid-plugin quickstart`
2. If you see `status: no_funds` / `low_balance` - get your deposit address and top up USDC on Arbitrum: `hyperliquid-plugin address`
3. If you see `status: needs_deposit` - bridge Arbitrum USDC into Hyperliquid (arrives in 2-5 min): `hyperliquid-plugin deposit --amount 50 --confirm`
4. One-time: bind your signing address so orders can be signed: `hyperliquid-plugin register`
5. If you see `status: ready` - place a perp order on the default DEX, fund a HIP-3 builder DEX for RWAs, OR fund USDH for HIP-4 outcomes: `hyperliquid-plugin order --coin BTC --side buy --size 0.001 --leverage 5 --confirm` OR `hyperliquid-plugin dex-transfer --to-dex xyz --amount 5 --confirm` (then trade `xyz:CL` / `xyz:NVDA` / etc.) OR `hyperliquid-plugin transfer --amount 12 --direction perp-to-spot --confirm` -> `hyperliquid-plugin usdh-fund --amount 11 --confirm` (then `outcome-list` to see what to bet on)
6. If you see `status: active` or `status: has_builder_dex_position` - review positions (pass `--dex xyz` for builder DEX positions) and attach stop-loss / take-profit: `hyperliquid-plugin positions --dex xyz` -> `hyperliquid-plugin tpsl --coin xyz:CL --sl-px 95 --tp-px 130 --confirm`
7. If you see `status: has_outcome_position` - review HIP-4 outcome holdings; settlement is AUTOMATIC at expiry (no claim/redeem needed), but you can sell early to lock in P&L: `hyperliquid-plugin outcome-positions` -> `hyperliquid-plugin outcome-sell --outcome <id> --side <yes|no> --shares N --price 0.001 --tif Ioc --confirm` (closes at touch)
8. Close a perp position: `hyperliquid-plugin close --coin xyz:CL --confirm` (works for both default and builder DEX coins via the `dex:symbol` prefix)
9. Withdraw USDC: `hyperliquid-plugin dex-transfer --from-dex xyz --amount 5 --confirm` (back to default DEX) -> `hyperliquid-plugin withdraw --amount 50 --confirm` (default DEX -> Arbitrum)
