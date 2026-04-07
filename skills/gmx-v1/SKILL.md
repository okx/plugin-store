---
name: gmx-v1
description: Trade perpetuals, swap tokens, and manage GLP liquidity on GMX V1 (Arbitrum/Avalanche). Supports token swaps, buying/selling GLP, opening/closing leveraged positions, and ERC-20 approvals via onchainos.
---

# GMX V1 Plugin

GMX V1 is a decentralized perpetuals and spot trading protocol on Arbitrum and Avalanche. Unlike GMX V2, V1 uses direct execution (no keeper delay) for swaps and GLP operations. Perpetual positions use a lightweight keeper with a 0.0001 ETH execution fee.

**Architecture:** Read ops call the GMX REST API. Write ops always ask user to confirm before submitting via `onchainos wallet contract-call` to GMX V1 contracts (Router, RewardRouter, PositionRouter).

**Key contracts (Arbitrum 42161):**
- Router: `0xaBBc5F99639c9B6bCb58544ddf04CF3C176D2B00`
- PositionRouter: `0xb87a436B93fE243ff3BC3ff12dA8dcFF7A5a36a7`
- GlpManager: `0x321F653eED006AD1C29D174e17d96351BDe22649`
- RewardRouter: `0xA906F338CB21815cBc4Bc87ace9e68c87eF8d8F1`

---

## Pre-flight Checks

1. Install the GMX V1 plugin binary:
   ```
   npx onchainos plugin install gmx-v1
   ```
2. Ensure onchainos is logged in with a funded wallet:
   ```
   onchainos wallet balance --chain 42161
   ```
3. For swaps: approve your input token to the Router first using `approve-token`.
4. For buying GLP: approve your input token to the GlpManager first.
5. For perp positions: ensure you have 0.0001 ETH available for the execution fee.

---

## Commands

### get-prices

Fetch current oracle prices for all tokens from GMX V1.

**Use case:** Check current token prices before trading or opening a position.

**Example:**
```
gmx-v1 get-prices --chain 42161
```

**Output:** Table of token symbols with min/max USD prices and token addresses.

---

### get-positions

Fetch all open perpetual positions for a wallet address.

**Use case:** View current leveraged positions including size, collateral, direction, and PnL.

**Example:**
```
gmx-v1 get-positions --chain 42161
gmx-v1 get-positions --chain 42161 --account 0xYourAddress
```

**Output:** Table of positions with market, direction (LONG/SHORT), size, collateral, and unrealized PnL.

---

### swap

Swap tokens using GMX V1 Router. No execution fee required - swap executes immediately.

**Use case:** Exchange one ERC-20 token for another via GMX V1 liquidity pools.

Before swapping, ask the user to confirm the transaction details. The swap is submitted via `onchainos wallet contract-call`.

**Example:**
```
gmx-v1 swap \
  --chain 42161 \
  --input-token 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --input-amount 10000000 \
  --output-token 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 \
  --min-output 0 \
  --dry-run
```

Remove `--dry-run` after user confirms to submit the transaction.

**Parameters:**
- `--input-token`: ERC-20 input token address
- `--input-amount`: Amount in token's smallest unit (e.g. 10000000 = 10 USDC with 6 decimals)
- `--output-token`: ERC-20 output token address
- `--min-output`: Minimum tokens to receive (0 = no slippage protection)

**Note:** Approve input token to Router (`0xaBBc5F99639c9B6bCb58544ddf04CF3C176D2B00`) before swapping.

---

### buy-glp

Buy GLP tokens by depositing ERC-20 tokens. No execution fee required.

**Use case:** Provide liquidity to GMX V1 by minting GLP tokens. GLP earns 70% of platform fees.

Before buying GLP, ask the user to confirm the transaction details. The transaction is submitted via `onchainos wallet contract-call`.

**Example:**
```
gmx-v1 buy-glp \
  --chain 42161 \
  --token 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --amount 5000000 \
  --min-usdg 0 \
  --min-glp 0 \
  --dry-run
```

Remove `--dry-run` after user confirms to submit the transaction.

**Parameters:**
- `--token`: ERC-20 token to deposit (e.g. USDC address)
- `--amount`: Amount to deposit in token's smallest unit
- `--min-usdg`: Minimum USDG to receive (slippage protection, 0 = none)
- `--min-glp`: Minimum GLP to receive (slippage protection, 0 = none)

**Note:** Approve input token to GlpManager (`0x321F653eED006AD1C29D174e17d96351BDe22649`) before buying GLP.

---

### sell-glp

Sell GLP tokens to receive ERC-20 tokens. No execution fee required.

**Use case:** Remove liquidity from GMX V1 by burning GLP tokens and receiving underlying tokens.

Before selling GLP, ask the user to confirm the transaction details. The transaction is submitted via `onchainos wallet contract-call`.

**Example:**
```
gmx-v1 sell-glp \
  --chain 42161 \
  --token-out 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --glp-amount 1000000000000000000 \
  --min-out 0 \
  --dry-run
```

Remove `--dry-run` after user confirms to submit the transaction.

**Parameters:**
- `--token-out`: ERC-20 token to receive
- `--glp-amount`: Amount of GLP tokens to burn (18 decimals, e.g. 1000000000000000000 = 1 GLP)
- `--min-out`: Minimum output tokens (slippage protection, 0 = none)

---

### open-position

Open a leveraged perpetual position (long or short). Requires 0.0001 ETH execution fee.

**Use case:** Enter a leveraged long or short position on a token using GMX V1 PositionRouter.

Ask the user to confirm position parameters before submitting. The transaction is submitted via `onchainos wallet contract-call`.

**Example:**
```
gmx-v1 open-position \
  --chain 42161 \
  --collateral-token 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --index-token 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 \
  --amount-in 5000000 \
  --size-usd 50.0 \
  --is-long true \
  --acceptable-price 2000000000000000000000000000000000 \
  --dry-run
```

Remove `--dry-run` after user confirms.

**Parameters:**
- `--collateral-token`: ERC-20 collateral token address (USDC for shorts; WETH for ETH longs)
- `--index-token`: Token to trade (e.g. WETH address)
- `--amount-in`: Collateral amount in token's smallest unit
- `--size-usd`: Total position size in USD (e.g. 50.0 for $50)
- `--is-long`: `true` for long, `false` for short
- `--acceptable-price`: Acceptable price from `get-prices` output in 30-decimal format
- `--execution-fee`: Override execution fee in wei (default: 100000000000000 = 0.0001 ETH)

**Warning:** 0.0001 ETH execution fee is sent with this transaction.

---

### close-position

Close a leveraged perpetual position (partial or full). Requires 0.0001 ETH execution fee.

**Use case:** Exit a long or short position by creating a decrease order on GMX V1 PositionRouter.

Ask the user to confirm before closing. The transaction is submitted via `onchainos wallet contract-call`.

**Example:**
```
gmx-v1 close-position \
  --chain 42161 \
  --collateral-token 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --index-token 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 \
  --size-usd 50.0 \
  --is-long true \
  --acceptable-price 1800000000000000000000000000000000 \
  --dry-run
```

Remove `--dry-run` after user confirms.

**Parameters:**
- `--collateral-token`: Collateral token address of the position
- `--index-token`: Token being traded (e.g. WETH)
- `--collateral-delta`: Collateral amount to withdraw (0 = leave in position)
- `--size-usd`: USD size to close (use full position size to close entirely)
- `--is-long`: `true` for long, `false` for short
- `--acceptable-price`: Acceptable price in 30-decimal GMX format
- `--min-out`: Minimum output tokens (0 = no slippage protection)
- `--withdraw-eth`: Set to `true` to receive ETH instead of WETH

**Warning:** 0.0001 ETH execution fee is sent with this transaction.

---

### approve-token

Approve an ERC-20 token for GMX V1 contracts (required before first swap or GLP purchase).

**Use case:** Set unlimited allowance for the GMX V1 Router or GlpManager.

Ask the user to confirm the approval before submitting. The approval is submitted via `onchainos wallet contract-call`.

**Example:**
```
gmx-v1 approve-token \
  --chain 42161 \
  --token 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --spender 0xaBBc5F99639c9B6bCb58544ddf04CF3C176D2B00 \
  --dry-run
```

Remove `--dry-run` after user confirms.

**Parameters:**
- `--token`: ERC-20 token address to approve
- `--spender`: Contract to approve (Router: `0xaBBc5F99639c9B6bCb58544ddf04CF3C176D2B00`; GlpManager: `0x321F653eED006AD1C29D174e17d96351BDe22649`)

---

## GLP Liquidity

GLP is the GMX V1 liquidity token. GLP holders:
- Earn 70% of all platform fees (in ETH/AVAX)
- Serve as counterparty to perp traders
- Can deposit/withdraw various tokens (ETH, WETH, USDC, USDT, DAI, etc.)

GLP price is determined by the total value of all tokens in the GLP pool.

---

## Execution Fees (Perpetual Positions Only)

GMX V1 requires a small ETH execution fee for perp position operations:
- `open-position`: 0.0001 ETH (100,000,000,000,000 wei)
- `close-position`: 0.0001 ETH (100,000,000,000,000 wei)
- `swap`, `buy-glp`, `sell-glp`: No execution fee

---

## Price Format

GMX V1 uses 30-decimal price format internally:
- `sizeDelta`: $1000 = `1000 * 10^30 = 1e33`
- API prices from `/prices/tickers`: `minPrice`/`maxPrice` are in 30-decimal format
- For ETH (18 decimals): `human_price = raw_price / 10^12`
- For USDC (6 decimals): `human_price = raw_price / 10^24`

---

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| `Cannot resolve wallet address` | onchainos not logged in | Run `onchainos wallet login` |
| `Unsupported chain ID` | Invalid chain specified | Use 42161 (Arbitrum) or 43114 (Avalanche) |
| `Invalid address` | Malformed token address | Verify address format (0x + 40 hex chars) |
| HTTP 429 / timeout | API rate limiting | Retry after a few seconds |
| `execution reverted` on swap | Token not approved | Run `approve-token` for Router first |
| `execution reverted` on buy-glp | Token not approved to GlpManager | Run `approve-token --spender GlpManager-addr` |

---

## Skill Routing

- For wallet balance: use `onchainos wallet balance`
- For token price checking: use `get-prices`
- For open positions: use `get-positions`
- For GMX V2 (keeper model, GM pools): use the `gmx-v2` skill
