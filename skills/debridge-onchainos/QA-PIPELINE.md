# QA pipeline for debridge-onchainos

Run AFTER `index.ts` is in place (it is — v0.1.0 is Form A complete).

## Pipeline overview

| Step | Skill | Live funds? | Status for v0.1.0 |
|---|---|---|---|
| 1. Static UX review | `explore-plugin` | ❌ | Run on each release |
| 2. Runnable build smoke | scaffold-emitted scripts | ❌ | Run before every test-plugin sweep |
| 3. Live DLN API wiring | direct REST probes via cli.ts | ❌ (DLN is read-only at this stage) | Run before every test-plugin sweep |
| 4. Regression sweep | `test-plugin` | ❌ | Required to mark green |
| 5. Live broadcast | `test-plugin` + onchainos | **✅ YES — OPT-IN ONLY** | Last step |

## Step 1 — Static UX review

**Skill:** `explore-plugin`
**Trigger:** "explore plugin debridge-onchainos"
**Live funds needed:** ❌ no

The skill simulates a brand-new user. For a Form A skill like this, it'll:
- Read SKILL.md fresh — note any confusing parts in the Command Index or routing-conversion table
- Read the sub-skills (`swap/`, `signing/`, `analytics/`) — verify the routing-conversion replacements are clearly documented
- Verify the example `window.ethereum.request({method:'eth_sendTransaction'})` patterns in `signing/metamask.md` lines 22-76 are clearly marked as REPLACED by OnchainOS routing
- Probe error code catalog clarity (DEBRIDGE_API_ERROR, ROUTE_NOT_FOUND, ORDER_NOT_FOUND, etc.)

Patch any findings to the main SKILL.md before proceeding.

## Step 2 — Runnable build smoke

```bash
cd ~/.claude/skills/debridge-onchainos
npm install
npx tsx cli.ts --version       # → 0.1.0
npx tsx cli.ts --help          # → lists 5 tools

# Read-only smoke
npx tsx cli.ts listSupportedChains '{}'

# getQuote (BSC USDT → Arbitrum USDC, 1 USDT)
npx tsx cli.ts getQuote '{
  "srcChainId":56,
  "srcChainTokenIn":"0x55d398326f99059fF775485246999027B3197955",
  "srcChainTokenInAmount":"1",
  "srcChainTokenInDecimals":18,
  "dstChainId":42161,
  "dstChainTokenOut":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831"
}'

# getOrderStatus (expected: ORDER_NOT_FOUND for a synthetic id)
npx tsx cli.ts getOrderStatus '{"orderId":"0x0000000000000000000000000000000000000000000000000000000000000001"}'

# Write-tool dry runs (no real broadcast; just observe the pending_sign envelope)
npx tsx cli.ts buildBridge '{
  "srcChainId":56,
  "srcChainTokenIn":"0x55d398326f99059fF775485246999027B3197955",
  "srcChainTokenInAmount":"1",
  "srcChainTokenInDecimals":18,
  "dstChainId":42161,
  "dstChainTokenOut":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
  "walletAddress":"<your-wallet>"
}'

npx tsx cli.ts buildSameChainSwap '{
  "chainId":56,
  "tokenIn":"0x55d398326f99059fF775485246999027B3197955",
  "tokenInAmount":"1",
  "tokenInDecimals":18,
  "tokenOut":"0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",
  "walletAddress":"<your-wallet>"
}'
```

Confirm:
- All tools return well-formed JSON.
- `listSupportedChains` lists ≥ 8 chains.
- `getQuote` returns `data.estimation.dstChainTokenOut.recommendedAmount`.
- `getOrderStatus` for a synthetic orderId returns `error_code: 'ORDER_NOT_FOUND'`.
- `buildBridge` / `buildSameChainSwap` return either an approve `pending_sign` (Step 1 of 2) OR the order `pending_sign` depending on the wallet's current allowance.

## Step 3 — Live DLN API wiring

DLN is read-only on these endpoints (create-tx returns calldata; nothing is broadcast). So you can validate the full request → response shape without a wallet:

```bash
# Run with a real wallet address to bypass DLN compliance flagging on test addresses
npx tsx cli.ts buildBridge '{"...","walletAddress":"<real-eoa>"}'
```

Decode the returned `unsigned_tx.data` (first 4 bytes is the selector) and confirm it matches the DLN router's expected entrypoint.

## Step 4 — Regression sweep

**Skill:** `test-plugin`
**Trigger:** "test plugin debridge-onchainos"
**Live funds needed:** ❌ no

Phases 0/1/2/3.5/5:
- Phase 0: version consistency (package.json ↔ SKILL.md frontmatter)
- Phase 1: doc cross-check (SKILL.md Command Index ↔ exports ↔ --help)
- Phase 2: all read-only tools, validate output shape + field completeness
- Phase 3.5: idempotency (repeat read-only calls)
- Phase 5: edge cases (validators, error variants — invalid orderId, unsupported chain, zero amount)

Patch Major+ findings before declaring done.

## Step 5 — Live broadcast (OPTIONAL)

**Skill:** `test-plugin --live`
**Trigger:** "test plugin debridge-onchainos --live --budget $10"
**Live funds needed:** ✅ YES

### Prerequisites
- Onchainos wallet logged in: `onchainos wallet status` → `loggedIn: true`
- Funded wallet on a supported chain (~$5–10 of bridgeable token + native gas)
- User has EXPLICITLY OPTED IN

### What it does

Picks the lowest-risk bridge route (e.g. 0.5 USDC BSC → Arbitrum):

1. Pre-flight: records nonce + token balance + DLN quote
2. Calls `buildBridge` → if allowance insufficient, receives approve `pending_sign`
3. Broadcasts approve via `onchainos wallet contract-call`
4. Polls for approve confirmation
5. Re-calls `buildBridge` → receives DLN order `pending_sign` (with `orderId`)
6. Broadcasts the DLN order tx
7. Polls `getOrderStatus` until status = `Fulfilled` (~1–3 min for most routes)
8. Reports: origin tx hash, destination tx hash, fill time, fee paid, expected vs actual amount

### Live broadcast pattern (single tx)

```bash
onchainos wallet contract-call \
  --chain <chain> \
  --to <unsigned_tx.to> \
  --input-data <unsigned_tx.data> \
  --amt <unsigned_tx.value> \
  --biz-type defi \
  --strategy debridge-onchainos \
  --force
```

## Skipping live tests

If a funded wallet is unavailable or the developer doesn't want to spend real
funds, **stop at Step 4**. The pipeline is designed so steps 1-4 produce a
serviceable plugin without any live requirement. Live tests can be deferred
indefinitely.

## When all 5 steps pass

The plugin is production-ready. Record the live tx hashes (from Step 5) in the
Migration note of SKILL.md for audit traceability.
