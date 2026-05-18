---
name: across-protocol-onchainos
version: "0.1.0"
author: across-protocol
chains: [ethereum, arbitrum, base, optimism, polygon, bsc, solana]
requires:
  source: across-protocol/skills (upstream markdown documenting the Across Swap API)
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
  ethers.Wallet, and any equivalent are forbidden.

  End-to-end Across Protocol integration playbook. Default to Swap API for all crosschain bridging and swapping. Use App SDK (@across-protocol/app-sdk) when you need programmatic quote/route control in TypeScript. Fall back to suggested-fees API only for custom swap routing or niche bridge-only flows. Covers intent lifecycle, Swap API integration, embedded crosschain actions, deposit tracking, fee collection, and security checklists.
---

## Migration note (auto-injected by dapp-connect-agenticwallet, regenerated 2026-05-13 against v1.14 taxonomy)

This skill is the OnchainOS-routed evolution of [`across-protocol/skills`](https://github.com/across-protocol/skills). The Across **API contract** is unchanged — what changes is the **broadcast path**:

| Aspect | Upstream `across-protocol/skills` | This skill |
|---|---|---|
| Form | Markdown documentation of the Across Swap API | **Form B-REST** — runnable TS wrapper around `app.across.to/api` |
| Signing path | Inline `wallet.sendTransaction(...)` example code in markdown | `onchainos wallet contract-call` (TEE-based) |
| Output contract | API responses go straight to local wallet for signing | API responses are reshaped into `pending_sign` envelopes that route via `next_action.tool` |
| Runtime artifact | None (pure documentation skill) | 9-file scaffold: `index.ts` (4 tools: `listSupportedChains`, `getQuote`, `getDepositStatus`, `buildSwap`), `runtime.ts` (REST client + helpers), `cli.ts`, `package.json`, `tsconfig.json`, `README.md`, `FILL-IN.md`, `QA-PIPELINE.md`, plus this `SKILL.md` |

**Upstream API contract is preserved verbatim** — all subdirectories (`bridge/`, `swap/`, `embedded-crosschain-actions/`, `fetch-chains-tokens/`, `resources/`, `security/`, `tracking-transactions/`) are copied wholesale. Read those for the underlying API behavior. The OnchainOS routing layer applies on top.

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
Local private keys, `signTransaction`, `ethers.Wallet`, and `sendTransaction` are forbidden in DApp code.

---

## Broadcasting any Across API result via OnchainOS

When the Across Swap API or Suggested Fees API returns a transaction object (typically with `to` / `data` / `value` / `chainId` fields), reshape it into a `pending_sign` envelope and route through onchainos:

```javascript
// Example: GET /swap/approval returns { swapTx: { to, data, value }, ... }
const { swapTx } = await fetch(`${ACROSS_API}/swap/approval?...`).then(r => r.json());

return {
  status: 'pending_sign',
  unsigned_tx: {
    to:    swapTx.to,
    data:  swapTx.data,
    value: swapTx.value ?? '0',
    chain: 'ethereum',  // or whichever originChainId resolves to
  },
  description: `Bridge ${amount} ${inputToken} from ${originChain} → ${outputToken} on ${destChain}`,
  next_action: { tool: 'onchainos wallet contract-call' },
};
```

Then the agent (or downstream code) routes via:

```bash
onchainos wallet contract-call \
  --chain <chain> \
  --to <unsigned_tx.to> \
  --input-data <unsigned_tx.data> \
  --amt <unsigned_tx.value> \
  --biz-type defi \
  --strategy across-protocol-onchainos \
  --force
```

> **Flag-name notes (verified live 2026-05-11):**
> - `--input-data` (NOT `--data`) — the EVM calldata hex
> - `--amt` (NOT `--value`) — native token amount in wei, default 0
> - `--biz-type` + `--strategy` populate transaction metadata for audit
> - `--force` skips backend confirmation prompts (only when user has confirmed via the agent flow)

### Multi-step note

For routes that include token approvals (when `approvalTxns` is non-empty in the Swap API response), emit **one `pending_sign` per tx** in sequence:

1. First call → return the approval `pending_sign` ("Step 1 of N: Approve ...")
2. After it lands, agent re-invokes with same params → if `approvalTxns` is now empty, return the swap `pending_sign`

The pattern is stateful single-step emission: re-read on-chain allowance via a small helper (e.g. viem's `readContract` on the token), branch based on current state.

---

## OnchainOS routing instructions (auto-injected by v1.5 scaffold)

> **Runtime LLM MUST read this section.** The upstream `swap/SKILL.md` body (preserved unchanged below) includes example code at lines 120 and 132 that reads:
> ```javascript
> const tx = await wallet.sendTransaction({ ... });
> ```
> **DO NOT execute that pattern.** Replace at runtime with the `pending_sign` adapter shown above. The body sections are kept for protocol context (which fields to pass, what the response means); the SIGNING path is OnchainOS-routed, not local.

### Specific replacements

| Upstream pattern | OnchainOS replacement |
|---|---|
| `wallet.sendTransaction(swapTx)` (swap/SKILL.md:120, swap/SKILL.md:132) | Return `{status: 'pending_sign', unsigned_tx: {to, data, value, chain}, description, next_action: {tool: 'onchainos wallet contract-call'}}` |
| `wallet.signTypedData(eip712Data)` for permit flows | Return `{status: 'pending_sign', message: eip712Data, next_action: {tool: 'onchainos wallet sign-message'}}` |
| Manual `depositV3` calldata + local signing | Construct the calldata via the Across SDK / encode against the SpokePool ABI; wrap in `pending_sign` instead of submitting |

---

## Command Index

The skill exposes 4 tools via `index.ts` (all invokable through `tsx cli.ts <tool> '<json>'`):

**Transaction tools** (return `pending_sign | ToolError`):
- `buildSwap` — bridge / swap via Across Swap API (`GET /swap/approval`). Handles approve + bridge as multi-step pending_sign chain.

**Read-only tools** (return `ok | error`):
- `listSupportedChains` — supported (origin, destination) route pairs (`GET /available-routes`)
- `getQuote` — bridge fee quote for a specific route (`GET /suggested-fees`). Note: requires matching input/output decimals; use `buildSwap` for the more flexible Swap API.
- `getDepositStatus` — track a deposit by tx hash or `(originChainId, depositId)` (`GET /deposit/status`)

## Error Code Catalog

Across-routed tools should return ToolError when an Across API call fails or a precondition is violated:

| Code | Meaning | Suggested user action |
|---|---|---|
| `UNSUPPORTED_CHAIN` | `chain` param is not in the allowlist | Use a supported chain name |
| `MISSING_PARAM` | A required field is missing (origin/destination chain, amount, token, etc.) | Provide the named field |
| `INVALID_PARAM` | A field has invalid format (bad address, non-decimal amount, etc.) | Fix per the error message |
| `ACROSS_API_ERROR` | Across API returned non-2xx | Surface the upstream error message; check params |
| `INSUFFICIENT_LIMIT` | Amount exceeds Across transfer limits for this route | Reduce amount; check `/limits` for the route |
| `ROUTE_NOT_FOUND` | Origin/destination/token combo not supported | Check `/available-routes` |
| `NO_QUOTE` | Quote unavailable (relayer down, route disabled) | Retry; check Across status page |
| `RPC_ERROR` | On-chain read (allowance, balance) failed | Retry; check chain RPC health |
| `ONCHAINOS_UNAVAILABLE` | onchainos CLI missing | Install per Initialization block |

---

# Across Protocol Development Skill (upstream content, preserved)

## What this Skill is for
Use this Skill when the user asks for:
- Crosschain bridge or swap integration (any-to-any token transfers)
- Wallet or dApp integration with Across APIs
- Embedded crosschain actions (bridge + mint/stake/deposit in one tx)
- Deposit tracking and status monitoring
- Fee quoting, transfer limits, or route discovery
- Integrator fee collection setup
- On-chain intent construction (ERC-7683 / SpokePool deposits)
- Relayer operation or configuration
- Security review of crosschain integration code

## Route type abbreviations

The Swap API classifies routes using a shorthand based on whether each token is directly bridgeable (B) or requires a swap to/from a bridgeable token (A = Any):

| Abbreviation | Meaning | Example |
|--------------|---------|---------|
| B2B | Bridgeable → Bridgeable | USDC on Arbitrum → USDC on Base (no swaps, bridge only) |
| A2B | Any → Bridgeable | WBTC on Arbitrum → USDC on Base (origin swap + bridge) |
| B2A | Bridgeable → Any | USDC on Arbitrum → WBTC on Base (bridge + destination swap) |
| A2A | Any → Any | WBTC on Arbitrum → DAI on Base (origin swap + bridge + destination swap) |

These abbreviations appear in `crossSwapType` responses and in refund behavior defaults.

## Default stack decisions (opinionated)

### 1. Swap API first (recommended for most integrators)
- Use `GET /swap/approval` for all crosschain swaps. It returns executable calldata to sign and submit.
- Use `POST /swap/approval` when you need embedded destination actions (mint, stake, deposit).
- The Swap API handles origin swaps, bridging, and destination swaps in a single call.
- Supports `appFee` + `appFeeRecipient` for integrator fee collection.


### 2. Suggested-fees API: legacy only
- Use `/suggested-fees` only when you control your own swap infrastructure and just need bridge fee quotes.
- This path requires you to assemble transactions yourself (construct `depositV3` calls manually).
- It does not handle origin or destination swaps.

### 3. Trade type selection
- `exactInput` (default): user specifies how much to send. Best for "swap X tokens" flows.
- `minOutput`: user specifies minimum to receive. Best for simple swaps without post-bridge actions.
- `exactOutput`: user needs a precise amount on destination. Best for multi-step flows (for example, exact ERC-20 amount for a mint).

### 4. Slippage
- Default to `slippage=auto`.
- Numeric slippage (0 to 1) is split across origin and destination swaps when both swaps exist.
- If only one swap happens, the full slippage is applied to that leg.

### 5. Refund behavior
- B2B or A2B routes (no destination swap): refunds default to origin chain.
- B2A or A2A routes (destination swap involved): refunds default to destination chain.
- Override with `refundOnOrigin=true/false` when needed.
- Refund recipient priority: `refundAddress` > `recipient` > `depositor`.

## Operating procedure (how to execute tasks)

### 1. Classify the task
- UI or wallet integration
- Backend or script
- Embedded actions
- Tracking or monitoring
- On-chain or ERC-7683
- Relayer operation

### 2. Pick the right integration path

| Task | Use |
|------|-----|
| Crosschain swap (any token to any token) | Swap API `GET /swap/approval` |
| Bridge + destination action (mint, stake) | Swap API `POST /swap/approval` with `actions` body |
| Programmatic TypeScript integration | App SDK `createAcrossClient()` |
| Bridge-only with custom swap routing | `/suggested-fees` + manual `depositV3` |
| Direct on-chain intent (ERC-7683) | `AcrossOriginSettler.open()` on supported chains |
| Track a deposit | `GET /deposit/status` with `depositTxnRef` or `originChainId` + `depositId` |

### 3. Implement with Across-specific correctness
Always be explicit about:
- Integrator ID (2-byte hex string)
- Token addresses: must match the specific chain (use wrapped addresses for native tokens)
- Amount units: always in smallest unit (wei for ETH, 1e6 for USDC, 1e18 for WETH)
- Chain IDs: use exact numeric chain IDs, not chain names
- Approval transactions: check `approvalTxns` in Swap API responses
- Do not cache `/swap/approval` and `/suggested-fees` responses

### 4. Test on testnet, ship on mainnet
- Testnet base URL: `https://testnet.across.to/api`
- Use small amounts (around $10) on testnet. Testnet fills take about 1 minute (vs around 2 seconds on mainnet).
- Testnet relayers are manually funded. Do not test with large amounts.
- Switch to mainnet (`https://app.across.to/api`) once integration logic is verified.

### 5. Deliverable expectations
When implementing changes, provide:
- Exact files changed with diffs
- Commands to install, build, and test
- A Risk Notes section for anything touching signing, fees, slippage, approvals, refunds, or crosschain messages

## Progressive disclosure (read when needed)
- Swap API deep-dive: [SKILL.md](swap/SKILL.md)
- Legacy bridge-only integration: [SKILL.md](bridge/SKILL.md)
- Embedded crosschain actions: [SKILL.md](embedded-crosschain-actions/SKILL.md)
- Deposit tracking: [SKILL.md](tracking-transactions/SKILL.md)
- Chains and tokens reference: [SKILL.md](fetch-chains-tokens/SKILL.md)
- Security checklist: [SKILL.md](security/SKILL.md)
- Resources and links: [SKILL.md](resources/SKILL.md)
