# debridge-onchainos

OnchainOS-routed deBridge DLN integration. Cross-chain bridging + same-chain
swaps via direct DLN REST calls. Replaces the upstream
[`debridge-finance/debridge-skills`](https://github.com/debridge-finance/debridge-skills)
MCP + local-signing flow with a typed `index.ts` tool surface that emits
`pending_sign` envelopes for `onchainos wallet contract-call`.

## Install / wire

This skill installs into `~/.claude/skills/debridge-onchainos/` and is picked
up automatically by Claude Code.

```
~/.claude/skills/debridge-onchainos/
├── SKILL.md           # AI guidance + Command Index + routing map
├── README.md          # this file
├── FILL-IN.md         # tool-by-tool implementation checklist (kept post-merge for reference)
├── QA-PIPELINE.md     # explore-plugin + test-plugin walkthrough
├── package.json
├── tsconfig.json
├── cli.ts             # dispatcher (tsx cli.ts <tool> '<json>')
├── runtime.ts         # debridgeApi REST client + helpers + CHAINS
├── index.ts           # 5 tools
├── analytics/         # upstream — DLN status semantics (REFERENCE only)
├── common/            # upstream — MCP setup (REFERENCE only)
├── signing/           # upstream — local-signing paths (REPLACED by onchainos)
├── swap/              # upstream — DLN order construction (REFERENCE only)
└── wallets/           # upstream — wallet discovery (REPLACED by onchainos)
```

## Example user prompt

> "Bridge 5 USDC from Arbitrum to Base using deBridge."

The agent will:

1. Call `buildBridge` with the route params.
2. If allowance to the DLN router is insufficient, receive a Step-1-of-2
   approve `pending_sign` and broadcast it via `onchainos wallet contract-call`.
3. Re-invoke `buildBridge` with the same params, receive the DLN order
   `pending_sign`, and broadcast it the same way.
4. Optionally call `getOrderStatus` with the `orderId` from the create-tx
   response to track destination fill.

## Tools

| Tool | Returns | Endpoint |
|---|---|---|
| `listSupportedChains` | `{status:'ok', data}` | `GET /v1.0/supported-chains-info` |
| `getQuote` | `{status:'ok', data}` | `GET /v1.0/dln/order/quote` |
| `getOrderStatus` | `{status:'ok', data}` | `GET /v1.0/dln/order/{orderId}` |
| `buildBridge` | `pending_sign \| ToolError` | `GET /v1.0/dln/order/create-tx` |
| `buildSameChainSwap` | `pending_sign \| ToolError` | `GET /v1.0/chain/transaction` |

## Live broadcast

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

## CLI smoke tests

```bash
cd ~/.claude/skills/debridge-onchainos
tsx cli.ts --help
tsx cli.ts listSupportedChains '{}'
tsx cli.ts getQuote '{
  "srcChainId":56,
  "srcChainTokenIn":"0x55d398326f99059fF775485246999027B3197955",
  "srcChainTokenInAmount":"1",
  "srcChainTokenInDecimals":18,
  "dstChainId":42161,
  "dstChainTokenOut":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831"
}'
```

## Provenance

Form A skill scaffolded from `debridge-finance/debridge-skills` (Form B,
MCP-routed) using the OnchainOS DApp scaffold v1.5. Upstream sub-skills
(`analytics/`, `common/`, `signing/`, `swap/`, `wallets/`) are preserved
unchanged as protocol-mechanics reference — **do not execute their
local-signing examples**.
