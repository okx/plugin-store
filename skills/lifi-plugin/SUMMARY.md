## Overview

LI.FI is a cross-chain bridge & swap aggregator over many underlying bridges (Across, Stargate, Mayan, Relay, etc.) and DEXs. This skill lets you list chains/tokens, get quotes, plan multi-hop routes, execute bridges/swaps with a single signed tx, and track in-flight transfers across Ethereum, Arbitrum, Base, Optimism, BSC, and Polygon.

## Prerequisites
- onchainos CLI installed and logged in (delivers signing for the source chain)
- A small amount of native gas token on whichever chain you bridge from (ETH on L1/L2s, BNB on BSC, POL on Polygon)
- USDC (or another bridgeable ERC-20) on at least one of the 6 supported chains, OR plan to bridge native (e.g. ETH -> ETH cross-chain)

## Quick Start
1. Check your current state and get a guided next step: `lifi-plugin quickstart`
2. If you see `status: rpc_degraded` - wait a minute then retry; more than half the public RPCs failed: `lifi-plugin quickstart`
3. If you see `status: no_funds` - wallet has nothing on any chain. Show your address per chain so you can top up: `lifi-plugin balance`
4. If you see `status: low_balance` - you have under $5 USDC anywhere. Verify and decide whether to top up or bridge native: `lifi-plugin balance --token USDC`
5. If you see `status: ready` - copy the recommended `next_command` (a 0.5 USDC bridge from your richest chain to a cheap L2): `lifi-plugin bridge --from-chain POL --to-chain BASE --from-token USDC --to-token USDC --amount 0.5 --confirm`
6. Plan alternatives before executing if you want to compare bridges (Across vs Mayan vs Stargate): `lifi-plugin routes --from-chain POL --to-chain BASE --from-token USDC --to-token USDC --amount 0.5 --limit 5 --order CHEAPEST`
7. After `bridge --confirm` returns a tx hash, track the cross-chain leg until DONE: `lifi-plugin status --tx-hash <h> --from-chain POL --to-chain BASE`
8. Verify the destination balance after the bridge resolves: `lifi-plugin balance --chain BASE --token USDC`
