# QA pipeline for lista-lending-onchainos v0.3.0

Two MANDATORY gates per v1.8 scaffold spec (P0-9). Live broadcast (G3) is opt-in.

| Step | Skill | Live funds? | Required? | Pass criterion |
|---|---|---|---|---|
| **G1. UX exploration** | `explore-plugin` | ❌ no | **MANDATORY** | No Major+ findings, or patched in same pass |
| **G2. Structured audit** | `test-skill` (language-agnostic) | ❌ no | **MANDATORY** | No Critical/Major findings, or patched + retested |
| G3. Live broadcast | `test-skill --live` | ✅ yes | OPT-IN | All write tools execute end-to-end |

> Note: `test-skill` replaces `test-plugin` for any scaffold output. The scaffold produces a tsx-based TypeScript Skill, and `test-skill`'s Phase −1 (Skill Invocation Profile detection) auto-adapts to that runtime.

## A1-A8 self-check expectations

| Check | Expected | Why |
|---|---|---|
| A1 (no unresolved template placeholders) | PASS | All templates substituted at copy time |
| A2 (valid YAML frontmatter) | PASS | Hand-validated v1.8 merge |
| A3 (3 fixed sections present) | PASS | `[Onchain OS dependency]`, `[signing constraint]`, `## Pre-flight Checks`, `## Signing Constraint` |
| A4 (no local-signing residue) | PASS | All wallet flows route through `next_action.tool` |
| A5 (pending_sign contract) | PASS | All 6 build* tools emit the contract |
| A6 (version sync) | PASS | SKILL.md `0.3.0` == package.json `0.3.0` |
| A7 (parseUnits decimals validated) | PASS | All decimals SDK-derived locals — no `parseUnits(_, params.X)` |
| A8 (native balance check on non-zero value) | SKIP or PASS | Lista lending operations are ERC-20 token flows; `unsigned_tx.value` is always `'0'` |

## Build smoke (no live funds)

```bash
cd ~/Documents/dapp-scaffold-work/lista-onchainos
npm install
npx tsx cli.ts --version
npx tsx cli.ts --help
LISTA_RUNTIME=mock npx tsx cli.ts listVaults '{"chain":"bsc"}'
```

## B-real read-only smoke (no funds; real RPC)

```bash
# vault list from real SDK
npx tsx cli.ts listVaults '{"chain":"bsc"}'

# vault position (replace 0xWALLET with any BSC address)
npx tsx cli.ts getVaultPosition '{"chain":"bsc","vaultAddress":"0x9a83cf80fb1d0d5e90547fef6d56b3afa7cdce4d","walletAddress":"0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9"}'

# market list
npx tsx cli.ts listMarkets '{"chain":"bsc"}'

# user holdings (note: Lista API can lag 30-60s behind on-chain state)
npx tsx cli.ts getHoldings '{"chain":"bsc","address":"0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9"}'

# simulate borrow (read-only — no tx emitted)
npx tsx cli.ts simulateBorrow '{"chain":"bsc","marketId":"0xd384584abf6504425c9873f34a63372625d46cd1f2e79aeedc77475cacaca922","walletAddress":"0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9"}'
```

## G3 (opt-in) — live broadcast smoke

Requires a funded `onchainos wallet` on BSC. Reproduces the 2026-05-11 v0.1.6 live validation flow.

```bash
# 1. Confirm wallet is funded
onchainos wallet balance --chain bsc

# 2. Build approve+deposit pending_sign (Step 1 of 2)
npx tsx cli.ts buildDeposit '{"chain":"bsc","vaultAddress":"0x9a83cf80fb1d0d5e90547fef6d56b3afa7cdce4d","amount":"1","walletAddress":"0xYOUR_ADDR"}'
# → returns approve pending_sign; sign via onchainos wallet contract-call

# 3. After approve tx lands, re-invoke for the deposit step
npx tsx cli.ts buildDeposit '{"chain":"bsc","vaultAddress":"0x9a83cf80fb1d0d5e90547fef6d56b3afa7cdce4d","amount":"1","walletAddress":"0xYOUR_ADDR"}'
# → returns deposit pending_sign; sign via onchainos wallet contract-call

# 4. After deposit lands, withdraw
npx tsx cli.ts buildWithdraw '{"chain":"bsc","vaultAddress":"0x9a83cf80fb1d0d5e90547fef6d56b3afa7cdce4d","amount":"1","walletAddress":"0xYOUR_ADDR"}'
# → returns redeem pending_sign; sign via onchainos wallet contract-call
```
