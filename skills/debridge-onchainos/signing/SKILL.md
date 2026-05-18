---
name: debridge-signing
description: >
  Sign and broadcast deBridge transactions. Routes to the correct signing
  method based on the detected signer: OWS local self-custody wallet,
  private key with ethers/viem, Foundry cast, browser wallet (MetaMask),
  or Privy embedded wallet. Use this skill whenever a deBridge transaction
  needs to be signed and sent on-chain — including bridge transactions, swap
  transactions, and token approvals. Also use when the user asks "how do I
  sign this transaction", "send the transaction", "approve the token",
  "broadcast to chain", "sign with OWS", "ows wallet", or needs help with
  EIP-712 typed data signing. This skill is called automatically by the
  bridge and swap workflows, but can also be used standalone for signing
  guidance.
license: MIT
metadata:
  author: deBridge
  version: "0.1.0"
---

# Transaction Signing

PREREQUISITE: Read ../common/SKILL.md for environment detection, auth, and chain configuration.

## Quick Reference

| Want to...                        | Go to                              |
|-----------------------------------|------------------------------------|
| Sign with OWS (recommended)       | [ows-signing.md](ows-signing.md)  |
| Sign with ethers.js or viem       | [sdk-signer.md](sdk-signer.md)    |
| Sign with Foundry cast            | [foundry-cast.md](foundry-cast.md)|
| Sign with MetaMask / browser      | [metamask.md](metamask.md)        |
| Sign via Privy MCP                | [privy-mcp.md](privy-mcp.md)      |
| Set up a wallet from scratch      | ../wallets/SKILL.md               |

## What Needs Signing

deBridge transactions from `mcp__debridge__create_tx` (cross-chain) or `mcp__debridge__transaction_same_chain_swap` (same-chain) return up to two transaction objects to sign:

1. **Token approval tx** (if allowance insufficient) — a standard EVM transaction calling `approve()` on the token contract.
2. **Bridge/swap tx** — an EVM transaction that may include EIP-712 typed data for DLN order creation.

Both are standard `{to, data, value, chainId}` objects. Sign and broadcast to the source chain RPC.

## Signer Routing

Use the `Signer` value from WALLET_DISCOVERY to select the right reference:

| Signer value    | Environment    | Read this file                       |
|-----------------|----------------|--------------------------------------|
| ows             | CLI            | [ows-signing.md](ows-signing.md)    |
| env-privkey     | CLI + Node.js  | [sdk-signer.md](sdk-signer.md)      |
| env-privkey     | CLI + cast     | [foundry-cast.md](foundry-cast.md)  |
| foundry-cast    | CLI            | [foundry-cast.md](foundry-cast.md)  |
| browser-wallet  | Browser        | [metamask.md](metamask.md)          |
| ethers-viem     | CLI / Headless | [sdk-signer.md](sdk-signer.md)      |
| mcp-wallet      | Any            | [privy-mcp.md](privy-mcp.md)        |
| none            | Any            | ../wallets/SKILL.md — set up first  |

### Resolving env-privkey

When Signer = `env-privkey`, a private key exists but a signing library is still needed. Pick based on what is available:

1. Node.js + ethers or viem installed → [sdk-signer.md](sdk-signer.md)
2. `cast` available → [foundry-cast.md](foundry-cast.md)
3. None of the above → install one: `npm install ethers` is the fastest path.

## Transaction Flow

After `mcp__debridge__create_tx` or `mcp__debridge__transaction_same_chain_swap` returns tx data:

### Step 0: Preflight Checks

Before signing, the agent MUST verify:

1. **Native balance for gas + fixFee** — Parse `fixFee` (wei) and `estimatedTransactionFee.total` from the response. Check that the wallet's native balance on the source chain covers both. If insufficient, stop and tell the user how much more they need.
2. **ERC-20 allowance** (EVM only, non-native tokens) — If the response does NOT include `approveTx`, the agent MUST still check the token's allowance for the bridge contract (`tx.to`). Use `scripts/erc20-approve.mjs` to check and approve if needed. Do NOT assume the MCP always returns `approveTx` — it may not.
3. **SOL rent/fees** (Solana source) — Solana bridge txs require ~0.024 SOL for rent deposits + tx fees, on top of the bridge amount. Check SOL balance before signing.

### Step 1: Check for Approval

If the response includes an approval transaction (`approveTx`):
1. Sign and send the approval tx first.
2. Wait for confirmation (1 block).
3. Proceed to Step 2.

If no `approveTx` but source token is ERC-20, run `scripts/erc20-approve.mjs` to check/approve (see Step 0).

### Step 2: Sign and Send Bridge Transaction

1. Take the main tx object (`tx` field from `create_tx` response).
2. Sign with the detected signer.
3. Broadcast to the source chain RPC.
4. Record the transaction hash.

### Step 3: Hand Off to Monitoring (cross-chain only)

For cross-chain bridges, pass the tx hash and order ID to ../swap/monitoring.md for order tracking. Same-chain swaps settle in a single transaction — no monitoring needed.

## RPC Endpoints

Most signers need an RPC connection to the source chain:
- OWS: for EVM, `ows sign tx` handles signing locally — broadcast via RPC separately; for Solana, set `SOLANA_RPC_URL` or use the default public RPC
- ethers/viem: pass RPC URL to provider constructor
- cast: use `--rpc-url` flag
- browser wallet: uses the wallet's connected RPC
- Privy MCP: handles RPC internally — no RPC URL needed from the agent

Use public RPCs or the user's configured RPC. Prefer user-provided RPCs or environment variables (`$ETH_RPC_URL`, `$RPC_URL`) over hardcoded defaults. The balance query skills include public RPCs as fallback defaults — override them when the user has configured RPCs.

For programmatic RPC discovery from Chainlist, read ../common/rpc-discovery.md.

## Common Errors

| Error                       | Cause                          | Fix                                            |
|-----------------------------|--------------------------------|-------------------------------------------------|
| Insufficient funds for gas  | Wallet has no native token     | Fund wallet with ETH/native token on source chain |
| Nonce too low               | Pending tx or state mismatch   | Wait for pending tx or reset nonce              |
| Transaction reverted        | Approval not confirmed yet     | Wait for approval confirmation before bridge tx |
| Invalid signature           | Wrong chain ID in signer       | Ensure signer chain ID matches source chain     |

## References

- [ows-signing.md](ows-signing.md) — OWS local self-custody signing (EVM, Solana, Tron)
- [sdk-signer.md](sdk-signer.md) — ethers.js and viem signing
- [foundry-cast.md](foundry-cast.md) — Foundry cast CLI signing
- [metamask.md](metamask.md) — Browser wallet signing
- [privy-mcp.md](privy-mcp.md) — Privy embedded wallet signing via MCP
