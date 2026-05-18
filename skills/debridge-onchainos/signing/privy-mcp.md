---
title: Signing via Privy MCP
impact: HIGH
impactDescription: "Server-side signing for zero-UI agent workflows via Privy embedded wallet"
tags: signing, privy, mcp, embedded-wallet, tee, eth_sendTransaction
---

# Privy MCP Signing

Privy MCP handles signing and broadcasting server-side. The agent passes transaction data directly to Privy's `eth_sendTransaction` tool — no local private key, no RPC URL, no signing library needed.

## Prerequisites

Privy MCP must be configured and connected. If not set up, read ../wallets/privy-embedded.md first.

Verify Privy MCP is available by checking for `mcp__privy__eth_sendTransaction` in the tool list.

## Sign and Send Transaction

Given tx data from `mcp__debridge__create_tx`:

```
Call mcp__privy__eth_sendTransaction:
  - to:       "0xeF4fB24aD0916217251F553c0596F8Edc630EB66"
  - value:    "1000000000000000"
  - data:     "0xb9303701..."
  - chain_id: 42161
```

The `create_tx` response fields map directly to Privy's parameters — no format conversion needed.

Privy signs with the embedded wallet's key (secured in TEE) and broadcasts to the chain's RPC. Returns the transaction hash.

## Approval Transaction

If `create_tx` returns an `approveTx`, send it first:

```
Call mcp__privy__eth_sendTransaction:
  - to:       <approveTx.to>
  - value:    "0"
  - data:     <approveTx.data>
  - chain_id: <source chain ID>
```

Wait for confirmation before sending the bridge/swap transaction.

## Sign EIP-712 Typed Data

If Privy MCP exposes a typed data signing tool (e.g., `mcp__privy__eth_signTypedData`), use it directly:

```
Call mcp__privy__eth_signTypedData:
  - domain:      <typedData.domain>
  - types:       <typedData.types>
  - primaryType: <typedData.primaryType>
  - message:     <typedData.message>
```

If the tool is not available, the standard `eth_sendTransaction` flow handles most deBridge operations without separate EIP-712 signing.

## Key Differences from Local Signers

| Aspect          | Local signers              | Privy MCP                         |
|-----------------|----------------------------|-----------------------------------|
| Private key     | Agent has access            | Never exposed — managed in TEE    |
| RPC connection  | Agent must provide          | Handled by Privy internally       |
| Gas estimation  | Agent must estimate         | Handled by Privy internally       |
| Nonce management| Agent must track            | Handled by Privy internally       |
| Chain switching | Agent must configure        | Pass `chain_id` per call          |

## Common Errors

| Error | Fix |
|-------|-----|
| Privy MCP tool not found | Privy MCP not configured — read ../wallets/privy-embedded.md |
| Insufficient funds | Fund Privy wallet with token + native gas on source chain |
| Invalid chain_id | Use standard EVM chain IDs (not deBridge internal IDs). For chains with deBridge-internal IDs (Sonic, Berachain, Neon, Gnosis, Mantle, Abstract), use the standard chain ID from the mapping table in ../common/chain-config.md |
| App Secret invalid | Regenerate at [dashboard.privy.io](https://dashboard.privy.io) and reconfigure MCP |
