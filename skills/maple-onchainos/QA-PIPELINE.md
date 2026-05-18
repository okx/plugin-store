# QA pipeline for maple-onchainos

Run AFTER `index.ts` is in place (it is — v0.1.0 is Form A complete).

## Pipeline overview

| Step | Skill | Live funds? | Status for v0.1.0 |
|---|---|---|---|
| 1. Static UX review | `explore-plugin` | ❌ | Run on each release |
| 2. Runnable build smoke | `tsx cli.ts` checks | ❌ | Required before every release |
| 3. Live RPC wiring | direct viem reads against mainnet | ❌ | Required before every release |
| 4. Regression sweep | `test-plugin` Phases 0/1/2/3/5 + A1-A8 self-checks | ❌ | Required to mark green |
| 5. Live broadcast | `test-plugin` + onchainos | **✅ YES — OPT-IN ONLY** | Last step |

## Step 1 — Static UX review

**Skill:** `explore-plugin`
**Trigger:** "explore plugin maple-onchainos"

Review:
- SKILL.md Command Index ↔ exports ↔ --help all in sync
- Withdrawal lifecycle described unambiguously (queue ≠ instant)
- Error-code catalog matches reality
- `prependOperatingExpenses`-style fee mutations called out? (n/a for Maple — see FILL-IN.md API quirks)

## Step 2 — Runnable build smoke

```bash
cd ~/.claude/skills/maple-onchainos
npm install
npx tsx cli.ts --version       # → 0.1.0
npx tsx cli.ts --help          # → lists 5 tools

# Read-only smoke
npx tsx cli.ts listSupportedPools '{}'
npx tsx cli.ts getPool '{"chain":"ethereum","poolAddress":"0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b"}'

# Sample addresses for live-read tests:
#   ethereum syrupUSDC: 0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b
#   ethereum syrupUSDT: 0x356b8d89c1e1239cbbb9de4815c39a1474d5ba7d

# Position read (use any large depositor's address — public knowledge from Etherscan top holders)
npx tsx cli.ts getPosition '{"chain":"ethereum","poolAddress":"0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b","walletAddress":"<addr>"}'

# Write-tool dry runs
npx tsx cli.ts buildDeposit '{"chain":"ethereum","poolAddress":"0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b","amount":"10","walletAddress":"<your-wallet>"}'
npx tsx cli.ts buildQueueWithdrawal '{"chain":"ethereum","poolAddress":"0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b","shares":"1","walletAddress":"<your-wallet>"}'
```

Confirm:
- All tools return well-formed JSON.
- `listSupportedPools` lists ≥ 2 pools.
- `getPool` returns `data.symbol === 'syrupUSDC'` (or syrupUSDT) and a non-zero `totalAssets`.
- `getPosition` returns shares=0 + suggestion note for a fresh wallet, OR positive shares + assets-equivalent.
- `buildDeposit` for a wallet with no allowance returns an approve `pending_sign` (Step 1 of 2).
- `buildQueueWithdrawal` for a wallet with no shares returns `NO_POSITION`.

## Step 3 — Live RPC wiring

Maple's writes are not testable read-only (deposit/redeem are state-changing), but the RPC reads (Steps 2's `getPool` and `getPosition` calls) ARE live reads against Ethereum mainnet. If those return expected shapes, the wiring is validated.

For broader confidence, compare `getPool.data.totalAssets` against the Syrup UI (https://syrup.fi) for a snapshot match.

## Step 4 — A1-A8 regression

```bash
cd ~/.claude/skills/maple-onchainos
# (uses the same A1-A8 block defined by the scaffold v1.6 self-check)
```

Phase 0: version consistency
Phase 1: SKILL.md ↔ exports ↔ --help cross-check
Phase 2: read-only output shape
Phase 3: idempotency (repeat read calls deterministic)
Phase 5: validators
A6: SKILL.md/package.json version sync
A7: every parseUnits decimals param validated upstream (n/a — pool registry supplies decimals; no user-passed decimals)
A8: nativeBalanceCheck wired for non-zero tx.value tools (passes for deposit/withdrawal even though both have value=0, since helper is imported)

## Step 5 — Live broadcast (OPTIONAL)

**Skill:** `test-plugin --live`

### Prerequisites
- Onchainos wallet logged in
- USDC on the chosen chain (≥ 1 USDC for a meaningful test on mainnet given pool min)
- Native gas (~$5 of ETH for mainnet; Base is much cheaper)
- User has EXPLICITLY OPTED IN

### What it does

For a 1 USDC deposit:

1. Pre-flight: record USDC balance + pool exchange rate
2. Call `buildDeposit` → if allowance insufficient, receive approve `pending_sign`
3. Broadcast approve via `onchainos wallet contract-call`
4. Poll for approve confirmation
5. Re-call `buildDeposit` → receive deposit `pending_sign`
6. Broadcast deposit tx
7. Poll `getPosition` until shares appear (~1 block)
8. Report: tx hashes, fee paid, shares received, assets-equivalent

For a withdrawal (optional reverse step):

1. Call `buildQueueWithdrawal`
2. Broadcast — receives requestRedeem tx
3. Note: assets do NOT arrive immediately — withdrawal manager processes in cycles

### Broadcast pattern

```bash
onchainos wallet contract-call \
  --chain <chain> \
  --to <unsigned_tx.to> \
  --input-data <unsigned_tx.data> \
  --amt <unsigned_tx.value> \
  --biz-type defi \
  --strategy maple-onchainos \
  --force
```

## Skipping live tests

If a funded wallet is unavailable or the developer doesn't want to spend real funds, **stop at Step 4**. Live tests are optional.

## When all 5 steps pass

Production-ready. Record live tx hashes in the SKILL.md's Migration note for audit traceability.
