# maple-onchainos

OnchainOS-routed Maple Finance / Syrup integration. Lend USDC/USDT into Maple's ERC-4626 pools (syrupUSDC, syrupUSDT) and queue withdrawals. EVM only (Ethereum mainnet + Base).

The skill reuses the calldata semantics from `maple-labs/maple-js` (the official Maple SDK) — `poolDeposit` maps to ERC-4626 `deposit(assets, receiver)`, `poolQueueWithdrawal` maps to `requestRedeem(shares, owner)` — but skips the SDK's provider-dependent gas estimation and local-signing helpers. Onchain OS handles those.

## Install / wire

This skill installs into `~/.claude/skills/maple-onchainos/` and is picked up automatically by Claude Code.

```
~/.claude/skills/maple-onchainos/
├── SKILL.md           # AI guidance + Command Index + routing map
├── README.md          # this file
├── FILL-IN.md         # per-tool implementation checklist
├── QA-PIPELINE.md     # explore-plugin + test-plugin walkthrough
├── package.json
├── tsconfig.json
├── cli.ts             # dispatcher (tsx cli.ts <tool> '<json>')
├── runtime.ts         # viem clients, pool registry, ABI fragments
└── index.ts           # 5 tools
```

## Example user prompt

> "Lend 100 USDC into Maple on Ethereum."

The agent will:

1. Call `buildDeposit` with `chain=ethereum`, `poolAddress=0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b` (syrupUSDC), `amount=100`, `walletAddress=...`.
2. Receive Step-1-of-2 approve `pending_sign`, broadcast via `onchainos wallet contract-call`.
3. Re-invoke `buildDeposit` with the same params, receive the deposit `pending_sign`, broadcast.
4. (Optional) Call `getPosition` to confirm the shares balance.

## Tools

| Tool | Returns | Calls |
|---|---|---|
| `listSupportedPools` | `{status:'ok', data}` | static pool registry |
| `getPool` | `{status:'ok', data}` | viem ERC-4626 reads |
| `getPosition` | `{status:'ok', data}` | viem ERC-20 / ERC-4626 reads |
| `buildDeposit` | `pending_sign \| ToolError` | viem encodeFunctionData → `deposit(uint256,address)` |
| `buildQueueWithdrawal` | `pending_sign \| ToolError` | viem encodeFunctionData → `requestRedeem(uint256,address)` |

## CLI smoke tests

```bash
cd ~/.claude/skills/maple-onchainos
tsx cli.ts --help
tsx cli.ts listSupportedPools '{}'
tsx cli.ts getPool '{"chain":"ethereum","poolAddress":"0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b"}'
tsx cli.ts getPosition '{"chain":"ethereum","poolAddress":"0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b","walletAddress":"<your-wallet>"}'
tsx cli.ts buildDeposit '{"chain":"ethereum","poolAddress":"0x80ac24aa929eaf5013f6436cda2a7ba190f5cc0b","amount":"10","walletAddress":"<your-wallet>"}'
```

## Live broadcast

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

## Provenance

Form A skill scaffolded from `maple-labs/maple-js` (TypeScript SDK, not a skill — built from scratch wrapping the SDK's calldata semantics) using the OnchainOS DApp scaffold v1.6 (incorporates P1-14/15/16 patterns: native-balance pre-flight, mandatory `validateDecimals`, fee-mutation flag docs).
