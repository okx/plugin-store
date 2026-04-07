---
name: pendle
description: "Pendle Finance yield tokenization plugin. Buy or sell fixed-yield PT tokens, trade YT yield tokens, provide or remove AMM liquidity, and mint or redeem PT+YT pairs. Trigger phrases: buy PT, sell PT, buy YT, sell YT, Pendle fixed yield, Pendle liquidity, add liquidity Pendle, remove liquidity Pendle, mint PT YT, redeem PT YT, Pendle positions, Pendle markets, Pendle APY. Chinese: 购买PT, 出售PT, 购买YT, 出售YT, Pendle固定收益, Pendle流动性, Pendle持仓, Pendle市场"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

## Architecture

- Read ops (list-markets, get-market, get-positions, get-asset-price) → direct REST calls to Pendle API; no wallet needed, no confirmation required
- Write ops (buy-pt, sell-pt, buy-yt, sell-yt, add-liquidity, remove-liquidity, mint-py, redeem-py) → after user confirmation, generates calldata via Pendle Hosted SDK, then submits via `onchainos wallet contract-call`
- ERC-20 approvals → checked from `requiredApprovals` in SDK response; submitted via `onchainos wallet contract-call` before the main transaction

## Supported Chains

| Chain | Chain ID |
|-------|---------|
| Ethereum | 1 |
| Arbitrum (default) | 42161 |
| BSC | 56 |
| Base | 8453 |

## Command Routing

| User intent | Command |
|-------------|---------|
| List Pendle markets / what markets exist | `list-markets` |
| Market details / APY for a specific pool | `get-market` |
| My Pendle positions / what do I hold | `get-positions` |
| PT or YT price | `get-asset-price` |
| Buy PT / lock fixed yield | `buy-pt` |
| Sell PT / exit fixed yield position | `sell-pt` |
| Buy YT / long floating yield | `buy-yt` |
| Sell YT / exit yield position | `sell-yt` |
| Add liquidity / become LP | `add-liquidity` |
| Remove liquidity / withdraw from LP | `remove-liquidity` |
| Mint PT+YT / tokenize yield | `mint-py` |
| Redeem PT+YT / burn for underlying | `redeem-py` |

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview the transaction without broadcasting
2. Show the user: amount in, expected amount out, implied APY (for PT), price impact
3. **Ask user to confirm** before executing on-chain
4. If price impact > 5%, issue a prominent warning before asking for confirmation
5. Execute only after explicit user approval
6. Report approve tx hash(es) (if any), main tx hash, and outcome

---

## Commands

### list-markets — Browse Pendle Markets

**Trigger phrases:** "list Pendle markets", "show me Pendle pools", "what Pendle markets are available", "Pendle market list"

```bash
pendle list-markets [--chain-id <CHAIN_ID>] [--active-only] [--skip <N>] [--limit <N>]
```

**Parameters:**
- `--chain-id` — filter by chain (1=ETH, 42161=Arbitrum, 56=BSC, 8453=Base); omit for all chains
- `--active-only` — show only active (non-expired) markets
- `--skip` — pagination offset (default 0)
- `--limit` — max results (default 20, max 100)

**Example:**
```bash
pendle list-markets --chain-id 42161 --active-only --limit 10
```

**Output:** JSON array of markets with `address`, `name`, `chainId`, `expiry`, `impliedApy`, `liquidity.usd`, `tradingVolume.usd`, PT/YT/SY token addresses.

---

### get-market — Market Details

**Trigger phrases:** "Pendle market details", "APY history for", "show me this Pendle pool"

```bash
pendle --chain <CHAIN_ID> get-market --market <MARKET_ADDRESS> [--time-frame <1D|1W|1M>]
```

**Parameters:**
- `--market` — market contract address (required)
- `--time-frame` — historical data window: `hour`, `day`, or `week`

**Example:**
```bash
pendle --chain 42161 get-market --market 0xd1D7D99764f8a52Aff0BC88ab0b1B4B9c9A18Ef4 --time-frame week
```

---

### get-positions — View Positions

**Trigger phrases:** "my Pendle positions", "what PT do I hold", "Pendle portfolio", "show my yield tokens"

```bash
pendle --chain <CHAIN_ID> get-positions [--user <ADDRESS>] [--filter-usd <MIN_USD>]
```

**Parameters:**
- `--user` — wallet address (defaults to currently logged-in wallet)
- `--filter-usd` — hide positions below this USD value

**Example:**
```bash
pendle get-positions --filter-usd 1.0
```

---

### get-asset-price — Token Prices

**Trigger phrases:** "Pendle PT price", "YT token price", "LP token value", "how much is this PT worth"

```bash
pendle get-asset-price [--ids <ADDR1,ADDR2>] [--asset-type <PT|YT|LP|SY>] [--chain-id <CHAIN_ID>]
```

**Example:**
```bash
pendle get-asset-price --ids 42161-0xPT_ADDRESS --chain-id 42161
```

**Note:** The `--ids` parameter requires chain-prefixed addresses in the format `<chainId>-<address>` (e.g. `42161-0x97c1a4ae3e0da8009aff13e3e3ee7ea5ee4afe84`). Use `list-markets` to get the correct prefixed IDs from the `pt`, `yt`, or `sy` fields.

---

### buy-pt — Buy Principal Token (Fixed Yield)

**Trigger phrases:** "buy PT on Pendle", "lock in fixed yield Pendle", "purchase PT token", "get fixed APY Pendle"

```bash
pendle --chain <CHAIN_ID> buy-pt \
  --token-in <INPUT_TOKEN_ADDRESS> \
  --amount-in <AMOUNT_WEI> \
  --pt-address <PT_TOKEN_ADDRESS> \
  [--min-pt-out <MIN_WEI>] \
  [--from <WALLET>] \
  [--slippage 0.01] \
  [--dry-run]
```

**Parameters:**
- `--token-in` — underlying token address to spend (e.g. USDC on Arbitrum: `0xaf88d065e77c8cc2239327c5edb3a432268e5831`)
- `--amount-in` — amount in wei (e.g. 1000 USDC = `1000000000`)
- `--pt-address` — PT token contract address from `list-markets`
- `--min-pt-out` — minimum PT to receive (slippage guard, default 0)
- `--from` — sender address (auto-detected if omitted)
- `--slippage` — tolerance, default 0.01 (1%)
- `--dry-run` — preview without broadcasting

**Execution flow:**
1. Run `--dry-run` to preview expected PT output and implied fixed APY
2. **Ask user to confirm** the trade before proceeding
3. Check `requiredApprovals` — if USDC approval needed, submit approve tx first
4. Execute: `onchainos wallet contract-call --chain <CHAIN_ID> --to <ROUTER> --input-data <CALLDATA> --force`
5. Return `tx_hash` confirming PT received

**Example:**
```bash
# Preview
pendle --chain 42161 buy-pt --token-in 0xaf88d065e77c8cc2239327c5edb3a432268e5831 --amount-in 1000000000 --pt-address 0xPT_ADDR --dry-run

# Execute (after user confirmation)
pendle --chain 42161 buy-pt --token-in 0xaf88d065e77c8cc2239327c5edb3a432268e5831 --amount-in 1000000000 --pt-address 0xPT_ADDR
```

---

### sell-pt — Sell Principal Token

**Trigger phrases:** "sell PT Pendle", "exit fixed yield position", "convert PT back to", "sell Pendle PT"

```bash
pendle --chain <CHAIN_ID> sell-pt \
  --pt-address <PT_ADDRESS> \
  --amount-in <PT_AMOUNT_WEI> \
  --token-out <OUTPUT_TOKEN_ADDRESS> \
  [--min-token-out <MIN_WEI>] \
  [--from <WALLET>] \
  [--slippage 0.01] \
  [--dry-run]
```

**Note:** If the market is expired, consider using `redeem-py` instead (avoids slippage for 1:1 redemption).

**Execution flow:**
1. Run `--dry-run` to preview output amount
2. **Ask user to confirm** — warn prominently if price impact > 5%
3. Check `requiredApprovals` — submit PT approval if needed
4. Execute: `onchainos wallet contract-call --chain <CHAIN_ID> --to <ROUTER> --input-data <CALLDATA> --force`
5. Return `tx_hash`

---

### buy-yt — Buy Yield Token (Long Floating Yield)

**Trigger phrases:** "buy YT Pendle", "long yield Pendle", "speculate on yield", "buy yield token"

```bash
pendle --chain <CHAIN_ID> buy-yt \
  --token-in <INPUT_TOKEN_ADDRESS> \
  --amount-in <AMOUNT_WEI> \
  --yt-address <YT_TOKEN_ADDRESS> \
  [--min-yt-out <MIN_WEI>] \
  [--from <WALLET>] \
  [--slippage 0.01] \
  [--dry-run]
```

**Execution flow:**
1. Run `--dry-run` to preview YT output
2. **Ask user to confirm** — remind user that YT is a leveraged yield position that decays to zero at expiry
3. Submit ERC-20 approval if required
4. Execute: `onchainos wallet contract-call --chain <CHAIN_ID> --to <ROUTER> --input-data <CALLDATA> --force`
5. Return `tx_hash`

---

### sell-yt — Sell Yield Token

**Trigger phrases:** "sell YT Pendle", "exit yield position", "convert YT back to"

```bash
pendle --chain <CHAIN_ID> sell-yt \
  --yt-address <YT_ADDRESS> \
  --amount-in <YT_AMOUNT_WEI> \
  --token-out <OUTPUT_TOKEN_ADDRESS> \
  [--min-token-out <MIN_WEI>] \
  [--from <WALLET>] \
  [--slippage 0.01] \
  [--dry-run]
```

**Execution flow:**
1. Run `--dry-run` to preview output amount
2. **Ask user to confirm** before executing
3. Submit YT approval if required
4. Execute: `onchainos wallet contract-call --chain <CHAIN_ID> --to <ROUTER> --input-data <CALLDATA> --force`
5. Return `tx_hash`

---

### add-liquidity — Provide Single-Token Liquidity

**Trigger phrases:** "add liquidity to Pendle", "become LP on Pendle", "provide liquidity Pendle", "deposit into Pendle pool"

```bash
pendle --chain <CHAIN_ID> add-liquidity \
  --token-in <INPUT_TOKEN_ADDRESS> \
  --amount-in <AMOUNT_WEI> \
  --lp-address <LP_TOKEN_ADDRESS> \
  [--min-lp-out <MIN_WEI>] \
  [--from <WALLET>] \
  [--slippage 0.005] \
  [--dry-run]
```

**Parameters:**
- `--lp-address` — LP token address from `list-markets` (market address is usually the LP token)

**Execution flow:**
1. Run `--dry-run` to preview LP tokens to receive
2. **Ask user to confirm** before adding liquidity
3. Submit input token approval if required
4. Execute: `onchainos wallet contract-call --chain <CHAIN_ID> --to <ROUTER> --input-data <CALLDATA> --force`
5. Return `tx_hash` and LP amount received

---

### remove-liquidity — Withdraw Single-Token Liquidity

**Trigger phrases:** "remove liquidity from Pendle", "withdraw from Pendle LP", "exit Pendle pool", "redeem LP tokens Pendle"

```bash
pendle --chain <CHAIN_ID> remove-liquidity \
  --lp-address <LP_TOKEN_ADDRESS> \
  --lp-amount-in <LP_AMOUNT_WEI> \
  --token-out <OUTPUT_TOKEN_ADDRESS> \
  [--min-token-out <MIN_WEI>] \
  [--from <WALLET>] \
  [--slippage 0.005] \
  [--dry-run]
```

**Execution flow:**
1. Run `--dry-run` to preview underlying tokens to receive
2. **Ask user to confirm** before removing liquidity
3. Submit LP token approval if required
4. Execute: `onchainos wallet contract-call --chain <CHAIN_ID> --to <ROUTER> --input-data <CALLDATA> --force`
5. Return `tx_hash`

---

### mint-py — Mint PT + YT from Underlying

**Trigger phrases:** "mint PT and YT", "tokenize yield Pendle", "split yield Pendle", "create PT YT"

```bash
pendle --chain <CHAIN_ID> mint-py \
  --token-in <INPUT_TOKEN_ADDRESS> \
  --amount-in <AMOUNT_WEI> \
  --pt-address <PT_ADDRESS> \
  --yt-address <YT_ADDRESS> \
  [--from <WALLET>] \
  [--slippage 0.005] \
  [--dry-run]
```

**Execution flow:**
1. Run `--dry-run` to preview PT and YT amounts to receive
2. **Ask user to confirm** the minting operation
3. Submit input token approval if required
4. Execute: `onchainos wallet contract-call --chain <CHAIN_ID> --to <ROUTER> --input-data <CALLDATA> --force`
5. Return `tx_hash`, PT minted, YT minted

---

### redeem-py — Redeem PT + YT to Underlying

**Trigger phrases:** "redeem PT and YT", "combine PT YT", "redeem Pendle tokens", "burn PT YT for underlying"

**Note:** PT must equal YT amount. Use this after market expiry for 1:1 redemption without slippage.

```bash
pendle --chain <CHAIN_ID> redeem-py \
  --pt-address <PT_ADDRESS> \
  --pt-amount <PT_AMOUNT_WEI> \
  --yt-address <YT_ADDRESS> \
  --yt-amount <YT_AMOUNT_WEI> \
  --token-out <OUTPUT_TOKEN_ADDRESS> \
  [--from <WALLET>] \
  [--slippage 0.005] \
  [--dry-run]
```

**Execution flow:**
1. Run `--dry-run` to preview underlying token to receive
2. **Ask user to confirm** the redemption
3. Submit PT and/or YT approvals if required
4. Execute: `onchainos wallet contract-call --chain <CHAIN_ID> --to <ROUTER> --input-data <CALLDATA> --force`
5. Return `tx_hash`

---

## Key Concepts

| Term | Meaning |
|------|---------|
| PT (Principal Token) | Represents the fixed-yield portion; redeems 1:1 for underlying at expiry |
| YT (Yield Token) | Represents the floating-yield portion; decays to zero at expiry |
| SY (Standardized Yield) | Wrapper around yield-bearing tokens (e.g. aUSDC) |
| LP Token | Pendle AMM liquidity position token |
| Implied APY | The current fixed yield rate locked in when buying PT |
| Market expiry | Date after which PT can be redeemed 1:1 without slippage |

## Troubleshooting

| Error | Likely cause | Fix |
|-------|-------------|-----|
| "Cannot resolve wallet address" | Not logged into onchainos | Run `onchainos wallet login` or pass `--from <address>` |
| "No routes in SDK response" | Invalid token/market address | Verify addresses using `list-markets` or Pendle docs |
| Tx reverts with slippage error | Price moved during tx | Increase `--slippage` (e.g. `--slippage 0.02`) |
| "requiredApprovals" approve fails | Insufficient token balance | Check balance with `onchainos wallet balance` |
| Market shows no liquidity | Market near expiry or low TVL | Use `list-markets --active-only` to find liquid markets |
