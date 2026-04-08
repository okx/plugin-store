---
name: four-meme
description: "Buy and sell meme tokens on Four.meme (BNB Chain bonding curve launchpad). Commands: tokens, info, buy, sell. Triggers: four meme, 4.meme, buy meme token, launch meme, four.meme buy, four meme sell"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Four.meme Plugin

Trade meme tokens on the Four.meme bonding curve launchpad on BNB Chain (BSC).

## Commands

### tokens
List supported base tokens and config from the Four.meme platform.

    four-meme tokens

### info
Get token details, current price, market cap, and bonding curve progress.

    four-meme info --token <TOKEN_ADDRESS>

### buy
Buy a meme token from the bonding curve using BNB.

    four-meme buy --token <TOKEN_ADDRESS> --amount-bnb 0.001
    four-meme buy --token <TOKEN_ADDRESS> --amount-bnb 0.001 --confirm

Without --confirm shows a preview. Add --confirm to broadcast on-chain.

### sell
Sell meme tokens back to the bonding curve for BNB.

    four-meme sell --token <TOKEN_ADDRESS> --amount <TOKEN_AMOUNT>
    four-meme sell --token <TOKEN_ADDRESS> --amount <TOKEN_AMOUNT> --confirm

Requires token approval before selling (handled automatically).

## Chain
BNB Chain (chain ID 56)

## Contracts
- TokenManager V2: 0x5c952063c7fc8610FFDB798152D69F0B9550762b
- TokenManagerHelper V3: 0xF251F83e40a78868FcfA3FA4599Dad6494E46034
