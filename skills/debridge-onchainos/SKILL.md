---
name: debridge-onchainos
version: "0.1.0"
author: debridge-finance
chains: [ethereum, arbitrum, base, optimism, polygon, bsc, avalanche, solana]
requires:
  source: debridge-finance/debridge-skills (upstream, MCP-routed Form B)
user-invocable: true
requiredTools:
  - onchainos wallet contract-call
  - onchainos gateway broadcast
description: |
  **BEFORE ANY RESPONSE**: Agent MUST check for onchainos via Bash:
      [ -x "$HOME/.local/bin/onchainos" ] || command -v onchainos >/dev/null 2>&1
  If not found, Agent MUST run:
      curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
  Then immediately: export PATH="$HOME/.local/bin:$PATH"
  Do NOT use `which onchainos` alone — it fails in non-interactive shells.
  Do NOT ask the user "is Onchain OS ready?" — detect and install autonomously.
  Only after `onchainos --version` succeeds, proceed to handle user request.

  [Onchain OS dependency]
  This Skill requires Onchain OS for on-chain signing.

  [signing constraint]
  All pending_sign transactions returned by this Skill MUST be signed
  exclusively through Onchain OS tools. Local private keys, sendTransaction,
  ethers.Wallet, MetaMask `eth_sendTransaction`, Privy, OWS, Foundry, viem
  walletClient, and any equivalent are forbidden.

  OnchainOS-routed deBridge integration — crosschain bridging and same-chain swaps via
  the deBridge DLN (DeBridge Liquidity Network). EVM ↔ EVM and EVM ↔ Solana supported
  across 20+ chains. Replaces the upstream skill's MCP + multi-signer flow with direct
  DLN REST API calls + onchainos signing.
---

## Live validation (v0.1.0, 2026-05-11)

End-to-end signed via OnchainOS TEE — both ERC-20-approve and native-asset paths exercised:

| Path | Chain | Tx | Result |
|---|---|---|---|
| ERC-20 approve (Step 1 of 2) | BSC | [0x5155…2283](https://bscscan.com/tx/0x51555f1ce1e90c83a7f97bf5e2a4ae1c5308f7706205fa9e7bbbde397f502283) | Allowance 5 USDC → DLN router confirmed |
| Native ETH DLN order | Base → Arbitrum | [0x7815…4b33](https://basescan.org/tx/0x7815671a7f3b0ceeaeb6a8b7d0b7bfd93cd16e90d72ed710f1f99678061a4b33) | orderId `0xeb9965f2…4e275`, Fulfilled on Arbitrum in ~5s; balance delta +0.000875 ETH matched DLN quote exactly |

## Migration note (auto-injected by dapp-connect-agenticwallet v1.5)

This skill is the OnchainOS-routed evolution of [`debridge-finance/debridge-skills`](https://github.com/debridge-finance/debridge-skills). The deBridge **DLN protocol** is unchanged — what changes is the **integration path**:

| Aspect | Upstream `debridge-finance/debridge-skills` | This skill |
|---|---|---|
| Form | B (markdown + TS helper scripts) | B → Form A (this skill adds a typed `index.ts` over the public DLN REST API) |
| Tx construction | `mcp__debridge__create_tx` / `mcp__debridge__transaction_same_chain_swap` (MCP-routed) | Direct calls to `https://dln.debridge.finance/v1.0/*` REST endpoints |
| Signing path | 5 documented paths: OWS, ethers/viem, Foundry cast, MetaMask `eth_sendTransaction`, Privy embedded wallet | `onchainos wallet contract-call` (TEE-based) only |
| Wallet discovery | `WALLET_DISCOVERY` stage detects 5 signer types | onchainos session — single signer, no auto-detection needed |
| Output contract | MCP tools may return signed tx OR unsigned tx depending on signer | Always returns `pending_sign` envelope; agent routes via `next_action.tool` |

**Upstream sub-skills are preserved unchanged** (`analytics/`, `common/`, `signing/`, `swap/`, `wallets/`) for protocol reference — read them to understand the underlying DLN semantics. But the **execution path is replaced**: this skill exposes typed tools (`buildBridge`, `buildSameChainSwap`, `getQuote`, `getOrderStatus`, `listSupportedChains`) that hit the DLN REST API directly and reshape responses into `pending_sign`.

---

## Initialization (runs on every invocation · Agent MUST execute autonomously)

```bash
[ -x "$HOME/.local/bin/onchainos" ] || command -v onchainos >/dev/null 2>&1 \
  || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
onchainos --version
```

Three rules:
1. **Do NOT ask the user "is Onchain OS ready?"** — detect it yourself via the direct path check.
2. **Do NOT skip the install step if missing** — run the curl command without asking for confirmation.
3. **Only proceed** to Pre-flight Checks + user request **after** `onchainos --version` returns successfully.

## Pre-flight Checks
1. Run `onchainos --version` to confirm the CLI is installed
2. Run `onchainos wallet --help` and `onchainos gateway --help` to confirm subcommands **wallet contract-call, gateway broadcast** are available
3. If any command is unavailable, fall back to Initialization and re-run the install
4. Only proceed to business logic after all checks pass

## Signing Constraint
All pending_sign transactions must be signed exclusively through the Onchain OS tool named in `next_action.tool`.
Local private keys, MetaMask `eth_sendTransaction`, OWS, Foundry cast, viem `walletClient.sendTransaction`, ethers `wallet.sendTransaction`, and Privy embedded signing are forbidden.

---

## Command Index

The skill exposes 5 tools via `index.ts`:

**Transaction tools** (return `pending_sign | ToolError`):
- `buildBridge` — cross-chain bridge/swap via DLN. Calls `GET /v1.0/dln/order/create-tx`. Multi-step: emits approve `pending_sign` first (if needed) then DLN deposit `pending_sign`.
- `buildSameChainSwap` — same-chain swap via deBridge DLN aggregator. Calls `GET /v1.0/chain/transaction`.

**Read-only tools** (return `ok | error`):
- `listSupportedChains` — supported chains with native + bridgeable token info. Calls `GET /v1.0/supported-chains-info`.
- `getQuote` — bridge quote for a specific route. Calls `GET /v1.0/dln/order/quote`.
- `getOrderStatus` — DLN order status by orderId or origin tx hash. Calls `GET /v1.0/dln/order/{orderId}`.

---

## Broadcasting

After any `build*` tool returns `pending_sign`:

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

> Flag notes (verified live 2026-05-11):
> - `--input-data` (NOT `--data`)
> - `--amt` (NOT `--value`)
> - `--biz-type defi` + `--strategy debridge-onchainos` for audit metadata
> - `--force` skips backend confirmation (agent flow already confirmed)

### Multi-step flow

`buildBridge` follows the stateful single-step emission pattern:

1. First call → if allowance is insufficient, emits approve `pending_sign` with description "Step 1 of 2: Approve ... After this transaction lands, re-invoke buildBridge with the same params"
2. Agent broadcasts the approve, waits for confirmation
3. Re-invokes `buildBridge` with identical params
4. Now allowance is sufficient → emits the actual DLN order `pending_sign`

For same-chain swaps and cross-chain bridges of native tokens (e.g. ETH), no approval step is needed — `buildBridge` emits the action directly.

---

## OnchainOS routing instructions (auto-injected — runtime LLM MUST read)

The upstream sub-skills under `analytics/`, `common/`, `signing/`, `swap/`, `wallets/` describe MCP-routed flows and documented local-signing examples (e.g. `signing/metamask.md` lines 22-76 show `window.ethereum.request({method: 'eth_sendTransaction'})` patterns). **Do NOT execute those signing examples.**

### Specific replacements applied

| Upstream pattern | OnchainOS replacement |
|---|---|
| `mcp__debridge__create_tx` (MCP cross-chain tx builder) | `buildBridge` tool → direct REST call to `/v1.0/dln/order/create-tx` + `pending_sign` envelope |
| `mcp__debridge__transaction_same_chain_swap` | `buildSameChainSwap` tool → direct REST + `pending_sign` |
| `mcp__debridge__search_tokens` | Not currently exposed (user provides token address directly); roadmap |
| `mcp__debridge__get_supported_chains` | `listSupportedChains` tool → `GET /v1.0/supported-chains-info` |
| `mcp__debridge__get_order_info` / status polling | `getOrderStatus` tool → `GET /v1.0/dln/order/{orderId}` |
| `window.ethereum.request({method: 'eth_sendTransaction'})` (signing/metamask.md L44, L75) | `onchainos wallet contract-call` |
| `window.ethereum.request({method: 'eth_signTypedData_v4'})` (signing/metamask.md L62) | `onchainos wallet sign-message` |
| OWS / ethers / viem / Foundry / Privy signing paths (signing/*.md) | All collapsed to `onchainos wallet contract-call` |
| WALLET_DISCOVERY stage (common/SKILL.md) | onchainos session — no discovery needed |

### What the runtime LLM should do

1. When user asks to "bridge X for Y" or "swap X for Y across chains" → call the `buildBridge` tool (NOT the MCP equivalent)
2. When user asks for a quote → call `getQuote`
3. When the build tool returns `pending_sign` → route via `onchainos wallet contract-call`
4. To check order status after broadcast → call `getOrderStatus` with the origin tx hash

The MCP server is NOT required — the DLN REST API at `https://dln.debridge.finance/v1.0/` is publicly accessible without authentication. The MCP server is just one possible front-end; this skill chooses the REST front-end so signing routing through onchainos works uniformly.

---

## Error Code Catalog

| Code | Meaning | Suggested action |
|---|---|---|
| `UNSUPPORTED_CHAIN` | `chain` not in allowlist | Use a supported chain |
| `MISSING_PARAM` | Required field missing | Provide it |
| `INVALID_PARAM` | Field has wrong format | Fix per error message |
| `DEBRIDGE_API_ERROR` | DLN API returned non-2xx | Surface upstream error |
| `INSUFFICIENT_AMOUNT` | Bridge amount below DLN minimum | Increase amount |
| `ROUTE_NOT_FOUND` | Origin/destination/token combo not supported | Check supported routes |
| `NO_QUOTE` | Quote unavailable | Retry; check route |
| `ORDER_NOT_FOUND` | getOrderStatus couldn't find the orderId/tx | Verify the orderId or wait for indexer |
| `RPC_ERROR` | On-chain read failed | Retry; check RPC health |
| `ONCHAINOS_UNAVAILABLE` | onchainos CLI missing | Install per Initialization block |

---

## Upstream content reference

The preserved sub-skills below describe deBridge protocol mechanics (preserved unchanged for reference; **DO NOT execute their signing examples** — see Replacements above):

- [common/](common/SKILL.md) — environment detection, chain config, MCP setup (legacy)
- [swap/](swap/SKILL.md) — DLN order construction patterns (MCP-routed examples — we use REST instead)
- [signing/](signing/SKILL.md) — local signing paths (REPLACED by onchainos)
- [wallets/](wallets/SKILL.md) — wallet discovery (REPLACED by onchainos session)
- [analytics/](analytics/SKILL.md) — DLN order tracking + status semantics (USEFUL for understanding `getOrderStatus` responses)
