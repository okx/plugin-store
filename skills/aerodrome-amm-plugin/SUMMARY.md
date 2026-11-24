## Overview

Aerodrome AMM is the classic automated market maker on Base, offering two pool types: volatile (xy=k) for uncorrelated assets and stable (stableswap) for pegged assets like USDC/USDT.

Core operations:
- Swap tokens through volatile or stable pools (auto-selects best output)
- Add and remove liquidity to earn trading fees
- Claim accrued LP trading fees

Tags: `defi` `base` `amm` `liquidity` `swap`

## Prerequisites

- No IP or region restrictions
- Supported chain: Base (8453)
- Supported tokens: WETH, USDC, USDT, AERO, DAI, cbETH, cbBTC, EURC, or any ERC-20 with an Aerodrome AMM pool
- onchainos CLI installed and authenticated with a Base wallet
- WETH or USDC on Base for swaps; both tokens required for liquidity provision

## Quick Start

1. **Connect**: run `onchainos wallet login your@email.com` and confirm your Base address with `onchainos wallet addresses --chain 8453`
2. **Check balance**: run `onchainos wallet balance --chain 8453` — you need at least one token for swaps
3. **Find a pool**: run `aerodrome-amm pools --token-a WETH --token-b USDC` to see available volatile and stable pools
4. **Get a quote**: run `aerodrome-amm quote --token-in WETH --token-out USDC --amount-in 0.01` to see expected output
5. **Preview swap**: run `aerodrome-amm swap --token-in WETH --token-out USDC --amount-in 0.01` — shows preview, no tx sent
6. **Execute**: add `--confirm` to broadcast: `aerodrome-amm swap --token-in WETH --token-out USDC --amount-in 0.01 --confirm`
