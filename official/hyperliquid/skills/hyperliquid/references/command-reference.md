# Hyperliquid CLI — Command Reference

Complete reference for all 11 commands, return fields, authentication, key concepts, and edge cases.

---

## Authentication

| Command Group | Auth Required | Method |
|---------------|---------------|--------|
| markets, spot-markets, price, orderbook, funding | No | Public API |
| buy, sell, cancel, positions, balances, orders | Yes | `EVM_PRIVATE_KEY` env var |

```bash
# Set in .env file (project directory or home)
EVM_PRIVATE_KEY=0x<your-private-key>
```

The private key signs Hyperliquid L1 actions via **EIP-712 phantom-agent scheme**:
1. Msgpack-encode the action + nonce + vault flag → keccak256 → `connectionId`
2. Build `Agent { source="a"(mainnet)/"b"(testnet), connectionId }` struct hash
3. EIP-712 digest → sign → `{ r, s, v }`

---

## 1. dapp-hyperliquid markets

List all perpetual futures markets with current mid prices and leverage limits.

```bash
dapp-hyperliquid markets
```

No parameters. No auth required.

**Return fields (per market):**

| Field | Description |
|-------|-------------|
| `symbol` | Asset symbol (e.g. BTC, ETH, SOL) |
| `mid_price` | Current mid price (string) |
| `szDecimals` | Size decimal precision — orders must respect this |
| `maxLeverage` | Maximum allowed leverage for this asset |

---

## 2. dapp-hyperliquid spot-markets

List all spot trading markets.

```bash
dapp-hyperliquid spot-markets
```

No parameters. No auth required.

**Return fields (per market):**

| Field | Description |
|-------|-------------|
| `name` | Market name (e.g. PURR/USDC) |
| `base` | Base token symbol |
| `quote` | Quote token symbol |
| `index` | Universe index (spot asset index = 10000 + index) |

---

## 3. dapp-hyperliquid price

Get the current mid price for a symbol.

```bash
dapp-hyperliquid price <symbol>
```

| Param | Required | Description |
|-------|----------|-------------|
| `<symbol>` | Yes | Asset symbol (e.g. BTC, ETH, SOL, PURR) |

No auth required.

**Return fields:**

| Field | Description |
|-------|-------------|
| `symbol` | Asset symbol |
| `mid_price` | Current mid price (best bid + best ask) / 2 |

**Error:** `symbol 'X' not found in allMids` — use `markets` or `spot-markets` to see valid symbols.

---

## 4. dapp-hyperliquid orderbook

Get the L2 order book snapshot for a symbol.

```bash
dapp-hyperliquid orderbook <symbol>
```

| Param | Required | Description |
|-------|----------|-------------|
| `<symbol>` | Yes | Asset symbol |

No auth required.

**Return fields:**

| Field | Description |
|-------|-------------|
| `coin` | Asset symbol |
| `levels[0]` | Bid levels (array of `{px, sz, n}`) — sorted best-to-worst |
| `levels[1]` | Ask levels (array of `{px, sz, n}`) — sorted best-to-worst |
| `time` | Timestamp (milliseconds) |

Per level: `px` = price, `sz` = total size, `n` = number of orders at that level.

---

## 5. dapp-hyperliquid funding

Get current and 24h historical funding rates for a symbol.

```bash
dapp-hyperliquid funding <symbol>
```

| Param | Required | Description |
|-------|----------|-------------|
| `<symbol>` | Yes | Asset symbol (perp only) |

No auth required.

**Return fields:**

| Field | Description |
|-------|-------------|
| `symbol` | Asset symbol |
| `current_funding` | Current funding rate from meta (may be null) |
| `history_24h` | Array of `{coin, fundingRate, premium, time}` for the past 24h |

**Interpreting funding rate:**
- Positive → longs pay shorts (bearish pressure)
- Negative → shorts pay longs (bullish pressure)
- Rate is per hour; annualized = rate × 8760

---

## 6. dapp-hyperliquid buy

Place a limit buy order (long perp or spot buy). Optionally set leverage first.

```bash
dapp-hyperliquid buy --symbol <symbol> --size <size> --price <price> [--leverage <leverage>]
```

| Param | Required | Default | Description |
|-------|----------|---------|-------------|
| `--symbol` | Yes | — | Asset symbol (e.g. BTC, ETH) |
| `--size` | Yes | — | Order size in base asset units (respect `szDecimals`) |
| `--price` | Yes | — | Limit price in USD |
| `--leverage` | No | Current account setting | Leverage multiplier (1–50, varies by asset) |

Requires `EVM_PRIVATE_KEY`.

**Return fields:**

| Field | Description |
|-------|-------------|
| `action` | "buy" |
| `symbol` | Asset symbol |
| `size` | Order size as submitted |
| `price` | Limit price as submitted |
| `leverage` | Leverage used (null if not set) |
| `result` | Raw Hyperliquid exchange response |

**Notes:**
- Price and size are normalized (trailing zeros stripped) before signing — required by Hyperliquid
- If `--leverage` is set, a separate `updateLeverage` action is submitted first (cross margin)
- Order type: GTC limit (`{"limit": {"tif": "Gtc"}}`)

---

## 7. dapp-hyperliquid sell

Place a limit sell order (short perp or spot sell).

```bash
dapp-hyperliquid sell --symbol <symbol> --size <size> --price <price>
```

| Param | Required | Description |
|-------|----------|-------------|
| `--symbol` | Yes | Asset symbol |
| `--size` | Yes | Order size in base asset units |
| `--price` | Yes | Limit price in USD |

Requires `EVM_PRIVATE_KEY`.

**Return fields:**

| Field | Description |
|-------|-------------|
| `action` | "sell" |
| `symbol` | Asset symbol |
| `size` | Order size as submitted |
| `price` | Limit price as submitted |
| `result` | Raw Hyperliquid exchange response |

---

## 8. dapp-hyperliquid cancel

Cancel an open order by symbol and order ID.

```bash
dapp-hyperliquid cancel --symbol <symbol> --order-id <order-id>
```

| Param | Required | Description |
|-------|----------|-------------|
| `--symbol` | Yes | Asset symbol the order was placed on |
| `--order-id` | Yes | Order ID (from `orders` command or buy/sell response) |

Requires `EVM_PRIVATE_KEY`.

**Return fields:**

| Field | Description |
|-------|-------------|
| `action` | "cancel" |
| `symbol` | Asset symbol |
| `order_id` | Order ID that was cancelled |
| `result` | Raw Hyperliquid exchange response |

---

## 9. dapp-hyperliquid positions

View all open perpetual positions for the wallet.

```bash
dapp-hyperliquid positions
```

No parameters. Requires `EVM_PRIVATE_KEY` (to derive wallet address).

**Return fields:**

| Field | Description |
|-------|-------------|
| `positions` | Array of open perp positions (`assetPositions`) |
| `margin_summary` | Account-level margin summary (total value, margin used, etc.) |
| `cross_margin_summary` | Cross-margin specific summary |

Per position fields (from Hyperliquid): `position.coin`, `position.szi` (size, negative = short), `position.entryPx`, `position.unrealizedPnl`, `position.leverage`, `position.liquidationPx`, `position.marginUsed`, `position.returnOnEquity`.

---

## 10. dapp-hyperliquid balances

View USDC perpetual margin balance and spot token balances.

```bash
dapp-hyperliquid balances
```

No parameters. Requires `EVM_PRIVATE_KEY`.

**Return fields:**

| Field | Description |
|-------|-------------|
| `perps_margin` | Margin summary for perpetuals account (accountValue, marginUsed, withdrawable) |
| `spot_balances` | Array of spot token balances `[{coin, hold, total, entryNtl}]` |

---

## 11. dapp-hyperliquid orders

List open orders, optionally filtered by symbol.

```bash
dapp-hyperliquid orders [--symbol <symbol>]
```

| Param | Required | Description |
|-------|----------|-------------|
| `--symbol` | No | Filter to a specific asset symbol |

Requires `EVM_PRIVATE_KEY`.

**Return fields:**

| Field | Description |
|-------|-------------|
| `orders` | Array of open orders |

Per order fields (from Hyperliquid): `coin`, `side` ("B"=buy/"A"=sell), `limitPx`, `sz`, `oid` (order ID), `timestamp`, `origSz`.

---

## Decimal Normalization

Hyperliquid normalizes price and size strings before hashing — the CLI does this automatically:

```
"0.170" → "0.17"
"58.00" → "58"
"100"   → "100"
```

Always match `szDecimals` from `markets` when specifying `--size`. Orders with extra decimal places are rejected.

---

## Common Workflows

### Research then Trade

```bash
dapp-hyperliquid markets                          # find symbol + maxLeverage
dapp-hyperliquid funding BTC                      # check funding (positive = bearish)
dapp-hyperliquid price BTC                        # get current mid price
dapp-hyperliquid orderbook BTC                    # check spread and depth
dapp-hyperliquid buy --symbol BTC --size 0.001 --price 70000 --leverage 10
dapp-hyperliquid positions                        # verify position opened
```

### Position Management

```bash
dapp-hyperliquid positions                        # see all open positions
dapp-hyperliquid orders                           # list pending orders
dapp-hyperliquid cancel --symbol BTC --order-id 123456
dapp-hyperliquid sell --symbol BTC --size 0.001 --price 71000
```

### Spot Trading

```bash
dapp-hyperliquid spot-markets                     # browse spot pairs
dapp-hyperliquid price PURR                       # check spot price
dapp-hyperliquid buy --symbol PURR --size 100 --price 0.09
```

---

## Edge Cases & Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `EVM_PRIVATE_KEY not set` | Missing env var | Add to `.env` file |
| `symbol 'X' not found` | Invalid symbol | Run `markets` or `spot-markets` first |
| `Rate limited` | Too many requests | Retry with backoff |
| Order rejected | Wrong `szDecimals` | Check precision via `markets` |
| Order rejected | Insufficient margin | Check `balances` first |
| Leverage > maxLeverage | Exceeds asset limit | Check `maxLeverage` from `markets` |
| Self-trade prevention | Would cross your own order | Cancel existing order first |

---

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `EVM_PRIVATE_KEY` | Trading commands | EVM wallet private key (with or without `0x` prefix) |
| `HYPERLIQUID_URL` | No | Override API base URL (default: `https://api.hyperliquid.xyz`). Set to testnet URL for testing. |
