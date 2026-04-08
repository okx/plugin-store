# four-meme

Buy and sell meme tokens on the Four.meme bonding curve launchpad on BNB Chain (BSC).

## Commands

### `tokens` — List supported base tokens
```bash
four-meme tokens
```
Fetches platform config from `https://four.meme/meme-api/v1/public/config` and lists
all supported quote tokens (BNB, CAKE, USDT, etc.) with graduation thresholds and fees.

### `info` — Token details
```bash
four-meme info --token 0xa0a8c195bd113fcd3592b03422e8b9a5fb2a4444
```
Shows token name, price, market cap, bonding curve progress, and on-chain status using
the Four.meme API and the TokenManagerHelper V3 contract.

### `buy` — Buy from bonding curve
```bash
# Preview (no broadcast)
four-meme buy --token 0xa0a8... --amount-bnb 0.001

# Broadcast
four-meme buy --token 0xa0a8... --amount-bnb 0.001 --confirm

# Custom slippage
four-meme buy --token 0xa0a8... --amount-bnb 0.001 --slippage-bps 200 --confirm
```

Uses `tryBuy` on the helper contract to get a price quote, then calls
`buyTokenAMAP(address,uint256,uint256)` on the TokenManager.

### `sell` — Sell back to bonding curve
```bash
# Preview
four-meme sell --token 0xa0a8... --amount 100000

# Broadcast (runs approve + sell in two transactions)
four-meme sell --token 0xa0a8... --amount 100000 --confirm
```

Automatically handles the ERC20 `approve` step before calling `sellToken`.

## Contracts (BSC, chain 56)

| Contract | Address |
|----------|---------|
| TokenManager V2 | `0x5c952063c7fc8610FFDB798152D69F0B9550762b` |
| TokenManagerHelper V3 | `0xF251F83e40a78868FcfA3FA4599Dad6494E46034` |
| TokenManager V1 (legacy) | `0xEC4549caDcE5DA21Df6E6422d448034B5233bFbC` |

## Notes

- All amounts use integer arithmetic — no floating point
- Buy amounts aligned to gwei precision (prevents "GW - GWEI" errors)
- Slippage defaults to 1% (100 bps)
- X Mode exclusive tokens are not supported (require platform signature)
- Graduated tokens (100% bonding curve) should be traded on PancakeSwap

## Live Test

Buy tx verified on BNB Chain:
`0x3fdce194c383cdf47dedaada82d8e8a3422ac5d47acd345a3d76bb321e03bdb5`
