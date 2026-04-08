# Four.meme Plugin Design

## Research Findings

### Protocol Overview
Four.meme is a meme token launchpad on BNB Chain (BSC) using a bonding curve model.
Tokens graduate to PancakeSwap once ~18 BNB is raised (bonding curve reaches 100%).
Trading fee: 1% (minimum 0.001 BNB). Total supply fixed at 1,000,000,000 tokens (1B).

### Contract Addresses (BSC, chain 56)

| Contract | Address |
|----------|---------|
| TokenManager V1 (legacy) | `0xEC4549caDcE5DA21Df6E6422d448034B5233bFbC` |
| TokenManager V2 (current) | `0x5c952063c7fc8610FFDB798152D69F0B9550762b` |
| TokenManagerHelper V3 | `0xF251F83e40a78868FcfA3FA4599Dad6494E46034` |
| AgentIdentifier | `0x09B44A633de9F9EBF6FB9Bdd5b5629d3DD2cef13` |

### API Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `https://four.meme/meme-api/v1/public/config` | GET | None | List all supported base tokens (BNB, CAKE, etc.) |
| `https://four.meme/meme-api/v1/private/token/get?address=<addr>` | GET | None | Get token info, price, market cap |

Note: Token listing API endpoints require login (EVM wallet signature). The plugin uses
on-chain queries via `tryBuy`/`trySell` helper for price quotes, and the token info API
for metadata.

### Buy/Sell Contract Functions

#### TokenManagerHelper V3 (read-only pre-calc)

```
tryBuy(address token, uint256 amount, uint256 funds)
  -> (tokenManager, quote, estimatedAmount, estimatedCost, estimatedFee, amountMsgValue, amountApproval, amountFunds)

trySell(address token, uint256 amount)
  -> (tokenManager, quote, funds, fee)

getTokenInfo(address token)
  -> (version, tokenManager, quote, lastPrice, tradingFeeRate, minTradingFee, launchTime, offers, maxOffers, funds, maxFunds, liquidityAdded)
```

Selector: `tryBuy(address,uint256,uint256)` = `0xe21b103a`
Selector: `trySell(address,uint256)` = `0xc6f43e8c`
Selector: `getTokenInfo(address)` = `0x1f69565f`

#### TokenManager V2 (write - buy/sell)

```
buyTokenAMAP(address token, uint256 funds, uint256 minAmount) payable
  -- spend `funds` BNB, get at least `minAmount` tokens

buyTokenAMAP(address token, address to, uint256 funds, uint256 minAmount) payable
  -- same but send tokens to `to`

sellToken(address token, uint256 amount)
  -- sell `amount` tokens (requires prior ERC20 approve)

sellToken(address token, uint256 amount, uint256 minFunds)
  -- sell with minimum BNB output
```

Selector: `buyTokenAMAP(address,uint256,uint256)` = `0x87f27655`
Selector: `buyTokenAMAP(address,address,uint256,uint256)` = `0x7f79f6df`
Selector: `sellToken(address,uint256)` = `0xf464e7db`
Selector: `sellToken(address,uint256,uint256)` = `0x3e11741f`

ERC20 `approve(address,uint256)` selector = `0x095ea7b3`

### Bonding Curve Mechanics

- Fixed total supply: 1,000,000,000 tokens
- 80% (800M tokens) for bonding curve sale
- Graduation threshold: ~18 BNB raised
- Upon graduation: 20% tokens + all raised BNB seeded into PancakeSwap
- Trading fee: 1% platform fee

### Notes for Implementation

1. V1 vs V2: Use `getTokenInfo` on the Helper contract to determine which TokenManager
   version controls a given token. The helper returns the correct `tokenManager` address.
2. For BNB-denominated tokens (quote = address(0)): send BNB as msg.value in buyTokenAMAP.
3. Token amounts use 18 decimals.
4. Amount precision must be aligned to GWEI (1e9) — round down to nearest gwei.
5. X Mode tokens (template & 0x10000 > 0) require a special encoded buy method.
