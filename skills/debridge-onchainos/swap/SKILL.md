---
name: debridge-swap
description: >
  Execute token swaps, bridges, and cross-chain transfers via deBridge DLN.
  Use this skill whenever the user wants to move, send, swap, exchange,
  trade, convert, or bridge tokens — whether on the same chain or across
  different chains (EVM-to-EVM, EVM-to-Solana, any L2-to-L2). This skill
  handles the full lifecycle: token lookup, quoting, amount conversion,
  balance checks, transaction signing, execution, and order tracking.
  IMPORTANT: Trigger this skill even for casual or indirect requests about
  moving tokens between wallets or chains. Common patterns include: "swap X
  for Y", "bridge tokens from A to B", "move my USDC to Base", "send ETH
  to Solana", "trade BNB for USDT on BSC", "how much to bridge from
  Polygon to Optimism", "convert DAI to USDC on Arbitrum", "exchange my
  tokens", "transfer to a cheaper L2", "what's the rate for swapping",
  "get me a quote", and any mention of moving crypto between chains or
  swapping one token for another. Also triggers for order monitoring: "my
  order is stuck", "check DLN order status", "track my bridge transaction",
  "order stuck on Created". If the user mentions specific token names
  (USDC, ETH, USDT, DAI, SOL, BNB, MATIC, AVAX) alongside any intent to
  move, send, swap, or trade them — use this skill.
license: MIT
metadata:
  author: deBridge
  version: "0.2.0"
---

# Token Swap & Bridge

PREREQUISITE: Read ../common/SKILL.md for environment detection, auth, and chain configuration.

## Quick Reference

| Want to...                    | Go to                                      |
|-------------------------------|--------------------------------------------|
| Look up chain IDs / tokens    | ../common/chain-config.md                  |
| Run preflight checks          | [preflight.md](preflight.md)               |
| Sign and send transaction     | ../signing/SKILL.md                        |
| Track cross-chain order       | [monitoring.md](monitoring.md)             |
| SDK workflow (coming soon)    | Not yet available — use MCP               |

## MCP Availability Check

Before starting, verify MCP is connected:

1. Call `mcp__debridge__get_supported_chains` (no parameters).
2. If it returns chain data → MCP is ready. Continue below.
3. If tool not found → MCP not connected. Set up the connection:
   - **Streamable HTTP (preferred):** `claude mcp add --transport http debridge https://agents.debridge.com/mcp`
   - **Stdio proxy (fallback):** `claude mcp add debridge npx -- -y @debridge-finance/debridge-mcp@latest`

   Both require restarting the session. For full setup details: read ACCESS_SETUP in ../common/SKILL.md or ../common/mcp-setup.md.

## Routing Decision

Determine whether this is a same-chain or cross-chain operation:

- **Source and destination on the same chain** → same-chain swap. Go to [Same-Chain Swap](#same-chain-swap).
- **Source and destination on different chains** → cross-chain swap/bridge. Go to [Cross-Chain Swap](#cross-chain-swap).

---

## Same-Chain Swap

### Step 1: Resolve Tokens

Look up token addresses and decimals on the swap chain.

```
Call mcp__debridge__search_tokens:
  - query: "USDC"              (NOT "search" — the parameter is "query")
  - chainId: "42161"           (string, NOT number)
```

Repeat for the output token. Record `address` and `decimals`.

### Step 2: Build Swap Transaction

```
Call mcp__debridge__transaction_same_chain_swap:
  - chainId:           "42161"               (string, chain deBridge ID)
  - tokenIn:           "0xaf88...e5831"      (input token address)
  - tokenInAmount:     "1000000000"          (amount in raw units — see Amount Conversion)
  - tokenOut:          "0x0000...0000"       (output token address)
  - tokenOutRecipient: "0xYourAddress"       (recipient address on the same chain)
```

Optional parameters:
- `slippage` — tolerance or `"auto"` (default: auto)
- `tokenOutAmount` — expected output or `"auto"`
- `senderAddress` — transaction submitter address
- `affiliateFeePercent` / `affiliateFeeRecipient` — affiliate fees

**All parameters are strings.** Do NOT pass numbers for chain IDs.

#### Response Format

The response includes:
- `tx` — the transaction object to sign and send (format depends on chain, see below)
- `tokenIn` / `tokenOut` — token metadata with `amount`, `minAmount`, `approximateUsdValue`
- `slippage` / `recommendedSlippage` — applied and recommended slippage (bps)
- `estimatedTransactionFee` — gas estimate with `total` (in raw native units) and `approximateUsdValue`
- `comparedAggregators` — price comparison with other DEX aggregators
- `costsDetails` — detailed fee breakdown

**EVM chains** — `tx` contains `{to, data, value}`:
```json
{
  "tx": { "to": "0x663D...c251", "data": "0x258c...", "value": "0" },
  "tokenOut": { "amount": "10027065", "minAmount": "10019037", ... }
}
```

**Solana** (chain ID `7565164`) — `tx` contains `{data}` only (hex-encoded serialized transaction):
```json
{
  "tx": { "data": "0x01000000..." },
  "tokenOut": { "amount": "87206972", "minAmount": "86927910", ... }
}
```
Solana transactions are fully serialized — pass `tx.data` to the Solana signing pipeline (see ../signing/ows-signing.md for OWS Solana flow).

### Step 3: Preflight Checks

Read [preflight.md](preflight.md) before proceeding.

Checks: balance, allowance (for ERC-20 input on EVM), and gas budget.

### Step 4: Sign and Send

Read ../signing/SKILL.md — it detects the available signer and routes to the correct signing method.

1. If ERC-20 input on EVM and allowance insufficient → sign and send approval tx first. Wait for 1 confirmation.
2. Sign and send the swap tx.

Same-chain swaps settle in a single transaction. No monitoring step needed — the tx receipt confirms completion.

---

## Cross-Chain Swap

Cross-chain swaps (also called bridges or cross-chain transfers) move tokens between different blockchains.

### Non-EVM Destinations

**Solana** (deBridge chain ID `7565164`):
- Recipient addresses are base58 (e.g., `Gh9ZwEm...`), not hex — validate format before calling `create_tx`.
- Native token (SOL) address: `11111111111111111111111111111111` (32 ones).
- Token amounts still use raw units but SOL has 9 decimals, not 18.

**Tron** (deBridge chain ID `100000026`):
- Recipient addresses are base58check (e.g., `T9yD14N...`), starting with `T` — not the same encoding as Solana.
- Native token (TRX) has 6 decimals.

For both: the source side (EVM) follows the normal EVM flow — approval if needed on the source chain, then sign and send. The `approveTx` from `create_tx` is always a source-chain operation.

### Step 1: Resolve Tokens

Look up token addresses and decimals on source and destination chains.

```
Call mcp__debridge__search_tokens:
  - query: "USDC"               (NOT "search" — the parameter is "query")
  - chainId: "1"                (string, NOT number)
```

Repeat for destination chain. Record `address` and `decimals` from the response.

For native tokens (ETH, BNB, etc.), use the zero address from ../common/chain-config.md.
For Solana native token (SOL), use `11111111111111111111111111111111`.

### Step 2: Build Transaction

```
Call mcp__debridge__create_tx:
  - srcChainId:                    "1"                    (string, source chain deBridge ID)
  - srcChainTokenIn:               "0xA0b8...eB48"       (source token address)
  - srcChainTokenInAmount:         "100000000"            (amount in raw units — see Amount Conversion)
  - dstChainId:                    "42161"                (string, destination chain deBridge ID)
  - dstChainTokenOut:              "0xaf88...e5831"       (destination token address)
  - dstChainTokenOutRecipient:     "0xRecipient..."       (recipient on destination chain)
  - srcChainOrderAuthorityAddress: "0xSender..."          (REQUIRED — sender's address on source chain)
  - dstChainOrderAuthorityAddress: "0xRecipient..."       (REQUIRED — recipient's address on destination chain)
```

**All parameters are strings.** Do NOT pass numbers for chain IDs.

Optional parameters:
- `dstChainTokenOutAmount` — expected output amount or `"auto"` for best quote (default: auto)
- `prependOperatingExpenses` — set `true` to add estimated operating expenses to the input amount
- `affiliateFeePercent` / `affiliateFeeRecipient` — affiliate fees

The response from `create_tx` includes:
- `tx` — the main transaction object (`to`, `data`, `value`, `chainId`)
- `approveTx` — token approval transaction (if ERC-20 allowance is insufficient)
- `orderId` — the DLN order ID for tracking
- Estimated output amount and fees

### Step 3: Preflight Checks

Read [preflight.md](preflight.md) before proceeding.

Preflight checks:
- Source token balance
- Token allowance (for ERC-20)
- Native token balance for gas
- Slippage tolerance

### Step 4: Sign and Send

Read ../signing/SKILL.md — it detects the available signer and routes to the correct signing method.

Execution order:
1. If `approveTx` is present → sign and send approval first. Wait for 1 confirmation.
2. Sign and send the main `tx`. Record the transaction hash.

### Step 5: Monitor

Read [monitoring.md](monitoring.md) to track the DLN order from creation to fulfillment on the destination chain.

---

## Amount Conversion

All amounts must be in raw units (smallest indivisible unit) as strings.

```
raw_units = human_amount × 10^decimals
```

Example: 100 USDC (6 decimals) → `"100000000"`

See ../common/chain-config.md for decimals and conversion table.

### Bundled Scripts

The `../common/scripts/` directory has TypeScript helpers that handle amount conversion, balance checks, allowances, and approvals. All scripts auto-discover RPC endpoints from Chainlist and read token decimals on-chain.

| Script | Purpose | Example |
|--------|---------|---------|
| `convert-amount.ts` | Convert human ↔ raw units | `npx tsx ../common/scripts/convert-amount.ts 100 0xA0b8...eB48 1` |
| `balance.ts` | Query native or ERC-20 balance | `npx tsx ../common/scripts/balance.ts 0xAddr 42161 --token 0xToken` |
| `allowance.ts` | Check ERC-20 allowance | `npx tsx ../common/scripts/allowance.ts 0xToken 0xOwner 0xSpender 1 --check 100` |
| `approve.ts` | Send ERC-20 approval tx | `npx tsx ../common/scripts/approve.ts 0xToken 0xSpender 1 --amount 1000` |
| `rpc.ts` | Discover RPC from Chainlist | `npx tsx ../common/scripts/rpc.ts 42161 --json` |

All scripts support `--json` for machine-readable output and `--rpc <url>` to override RPC discovery.

## Common Errors

| Error                        | Cause                              | Fix                                          |
|------------------------------|------------------------------------|----------------------------------------------|
| No route found               | Token pair not supported on chains | Check `get_supported_chains` and `search_tokens` |
| Insufficient allowance       | Preflight should catch this        | Send approval tx first, then retry           |
| Slippage exceeded            | Price moved during execution       | Re-call `create_tx` or `transaction_same_chain_swap` for fresh quote |
| Amount too small             | Below minimum amount               | Increase amount or check minimum             |
| Insufficient liquidity       | Pool too small for amount          | Reduce amount or try a different pair        |
| Destination chain unsupported| Wrong chain ID format              | Use deBridge chain IDs from ../common/chain-config.md |

