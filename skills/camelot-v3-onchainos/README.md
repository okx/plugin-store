# camelot-v3-onchainos

OnchainOS-routed Camelot V3 (Algebra-based concentrated liquidity) DEX integration on Arbitrum. Exact-input single-pool swaps via the deployed Algebra SwapRouter; quotes via the Algebra Quoter.

The official `CamelotLabs/camelot-sdk` (v0.0.8) ships only a Logger and Redis client — no swap helpers. This skill builds calldata directly with viem against the on-chain ABIs.

## Install / wire

```
~/.claude/skills/camelot-v3-onchainos/
├── SKILL.md
├── README.md
├── FILL-IN.md
├── QA-PIPELINE.md
├── package.json
├── tsconfig.json
├── cli.ts
├── runtime.ts
└── index.ts
```

## Example user prompt

> "Swap 1 USDC for WETH on Camelot V3."

The agent will:

1. Call `getTokenInfo` to learn USDC + WETH decimals (or it knows them).
2. Call `getQuote` with `tokenIn=USDC`, `tokenOut=WETH`, `amountIn=1`, `amountInDecimals=6` → returns `amountOut` and dynamic fee.
3. Call `buildSwap` with the same params + `walletAddress` → if allowance insufficient, receive Step-1 approve `pending_sign`.
4. Broadcast approve via `onchainos wallet contract-call`.
5. Re-invoke `buildSwap` → receive the exactInputSingle `pending_sign`. Broadcast.

## Tools

| Tool | Returns | Calls |
|---|---|---|
| `listSupportedChains` | `{status:'ok', data}` | static registry |
| `getTokenInfo` | `{status:'ok', data}` | viem ERC-20 reads |
| `getQuote` | `{status:'ok', data}` | viem simulateContract → Algebra Quoter |
| `buildSwap` | `pending_sign \| ToolError` | viem encodeFunctionData → exactInputSingle |

## CLI smoke

```bash
cd ~/.claude/skills/camelot-v3-onchainos
tsx cli.ts --help
tsx cli.ts listSupportedChains '{}'
tsx cli.ts getTokenInfo '{"chain":"arbitrum","tokenAddress":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831"}'
tsx cli.ts getQuote '{
  "chain":"arbitrum",
  "tokenIn":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
  "tokenOut":"0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
  "amountIn":"1",
  "amountInDecimals":6
}'
tsx cli.ts buildSwap '{
  "chain":"arbitrum",
  "tokenIn":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
  "tokenOut":"0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
  "amountIn":"1",
  "amountInDecimals":6,
  "walletAddress":"<your-wallet>",
  "slippageBps":100
}'
```

Reference token addresses on Arbitrum:
- USDC: `0xaf88d065e77c8cC2239327C5EDb3A432268e5831`
- WETH: `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1`
- ARB:  `0x912CE59144191C1204E64559FE8253a0e49E6548`
- USDT: `0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9`

## Live broadcast

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

## Provenance

Form A skill built from scratch wrapping the Algebra SwapRouter + Quoter ABIs (CamelotLabs/camelot-sdk lacks swap helpers). Scaffolded with the OnchainOS DApp scaffold v1.7 (P1-14 nativeBalanceCheck, P1-15 validateDecimals, P2-15 reliable RPC defaults).
