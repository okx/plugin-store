# QA pipeline for camelot-v3-onchainos

| Step | Skill | Live funds? | Status for v0.1.0 |
|---|---|---|---|
| 1. Static UX review | `explore-plugin` | ❌ | Run on each release |
| 2. Runnable build smoke | `tsx cli.ts` checks | ❌ | Required |
| 3. Live RPC wiring | viem reads + Quoter simulate | ❌ | Required |
| 4. Regression sweep | `test-plugin` Phases 0/1/2/3/4/5 + A1-A8 | ❌ | Required |
| 5. Live broadcast | `test-plugin --live` | ✅ OPT-IN | Last step |

## Step 1 — Static UX review

Verify:
- 4 tools cross-referenced consistently across SKILL.md / --help / index.ts (anchored Phase 1 regex per P2-14)
- Algebra-vs-Uniswap-V3 difference clearly noted (no `fee` field in exactInputSingle struct)
- Error-code catalog matches reality

## Step 2 — Runnable build smoke

```bash
cd ~/.claude/skills/camelot-v3-onchainos
npm install
npx tsx cli.ts --version       # → 0.1.0
npx tsx cli.ts --help          # → lists 4 tools

# read-only
npx tsx cli.ts listSupportedChains '{}'
npx tsx cli.ts getTokenInfo '{"chain":"arbitrum","tokenAddress":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831"}'
npx tsx cli.ts getQuote '{
  "chain":"arbitrum",
  "tokenIn":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
  "tokenOut":"0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
  "amountIn":"1",
  "amountInDecimals":6
}'

# write dry-run
npx tsx cli.ts buildSwap '{
  "chain":"arbitrum",
  "tokenIn":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
  "tokenOut":"0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
  "amountIn":"1",
  "amountInDecimals":6,
  "walletAddress":"<your-wallet>"
}'
```

Confirm:
- All tools return well-formed JSON.
- `getQuote` USDC→WETH returns positive amountOut (~$1 of WETH = ~0.00029 ETH at current prices).
- `buildSwap` with no allowance returns approve Step 1 of 2.

## Step 3 — Live RPC wiring

Read tests are live RPC against Arbitrum mainnet via publicnode. Compare USDC→WETH quote against `camelot.exchange` UI to spot-check rate.

## Step 4 — A1-A8 regression

```
A1 (no unresolved scaffold placeholders): PASS
A2 (YAML):               PASS
A3 (fixed sections):     PASS
A4 (no local signing):   PASS
A5 (pending_sign):       PASS
A6 (version sync):       PASS
A7 (validateDecimals):   PASS (amountInDecimals validated in getQuote + buildSwap)
A8 (native-balance):     PASS (helper imported; value=0 swaps no-op the check)
```

Plus test-plugin Phase 0/1/2/3/4/5 — all green.

## Step 5 — Live broadcast (OPT-IN)

### Prerequisites
- Onchainos logged in
- ≥ 0.5 USDC + ~0.001 ETH gas on Arbitrum
- Explicit opt-in

### Test sequence

1. Quote 0.5 USDC → WETH
2. `buildSwap` → receive approve `pending_sign`
3. Broadcast approve
4. Wait for confirmation, re-invoke `buildSwap` → swap `pending_sign`
5. Broadcast swap
6. Verify WETH balance increased by ~quoted amountOut on `getTokenInfo` for WETH

### Broadcast pattern

```bash
onchainos wallet contract-call \
  --chain arbitrum \
  --to <unsigned_tx.to> \
  --input-data <unsigned_tx.data> \
  --amt <unsigned_tx.value> \
  --biz-type defi \
  --strategy camelot-v3-onchainos \
  --force
```

## When all 5 steps pass

Production-ready. Record live tx hashes in SKILL.md's Migration note for audit traceability.
