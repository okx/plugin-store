---
title: deBridge Signing with OWS (Open Wallet Standard)
impact: HIGH
impactDescription: "Local self-custody signing for deBridge transactions and messages via OWS SDK/CLI"
tags: ows, signing, self-custody, local-key, evm, solana, tron, deBridge
---

# deBridge Signing with OWS

Local self-custody signing for deBridge transactions and messages using OWS wallets. Keys stay encrypted on the user's machine and are decrypted only in-process during signing — never exposed.

OWS supports nine chain families. The ones that overlap with deBridge, sorted by relevance:

| Chain family | OWS signing | deBridge support | Notes |
|--------------|-------------|------------------|-------|
| EVM          | `ows sign tx` / `ows sign message` | All EVM chains (Ethereum, Arbitrum, Base, Polygon, etc.) | Standard EVM tx signing works directly |
| Solana       | `ows sign message --encoding hex` | Solana (chainId `7565164`) | Needs scripted format adaptation — see below |
| Tron         | `ows sign tx` / `ows sign message` | Tron (chainId `100000026`) | base58check addresses, 6-decimal TRX |

OWS also supports Bitcoin, Cosmos, TON, Sui, Spark, and Filecoin — these are not yet supported by deBridge.

## EVM Sign + Broadcast

For EVM chains, `ows sign tx` returns a **raw signature** (`{signature, recovery_id}`), not a broadcast-ready signed transaction. You must assemble the signed tx before broadcasting. The bundled script handles this:

```bash
# Agent calls MCP to get the quote, then pipes create_tx JSON to the script
echo '<create_tx JSON response>' | node scripts/debridge-evm-bridge.mjs <wallet_name> --src-chain 137
```

The agent MUST call `mcp__debridge__create_tx` first, then pipe the JSON response to this script. The script handles: build unsigned EIP-1559 tx → sign with `ows sign tx` CLI → extract r/s/v from signature → assemble → broadcast → wait for confirmation. It also handles `approveTx` (ERC-20 allowance) automatically.

**Do NOT call MCP from inside JS scripts.** The agent is the MCP client — scripts only handle signing and broadcasting.

### OWS EVM signature format

`ows sign tx --json` returns:
```json
{"signature": "<128-char hex>", "recovery_id": 0}
```
- `r` = first 64 hex chars, `s` = next 64 hex chars, `v` = `recovery_id` (0 or 1)
- These must be attached to an ethers.js `Transaction` object via `tx.signature = Signature.from({r, s, v})` before broadcasting `tx.serialized`

---

## Solana Sign + Broadcast Pipeline

The rest of this section covers the Solana-specific signing pipeline.

Solana requires a scripted pipeline because of three format differences that the script handles automatically:
1. **Blockhash insertion** — deBridge returns a placeholder (all zeros) since blockhashes expire in ~60–90s; the script fetches a fresh one.
2. **Byte-range extraction** — Solana V0 signatures cover only the message bytes (offset 65+), not the full serialized tx; the script extracts the right range for OWS `signMessage()`.
3. **Encoding conversion** — OWS returns a hex signature; Solana RPC expects base64; the script converts before broadcast.

Bundled scripts in `scripts/`:

| Script | What it does | Install |
|--------|--------------|---------|
| `scripts/debridge-solana-bridge.mjs` | **Sign + broadcast**: reads create_tx JSON from stdin → sign → broadcast (recommended) | `npm install @open-wallet-standard/core` |
| `scripts/ows-solana-sign.mjs` | Sign-only: takes tx hex, signs and broadcasts | `npm install @open-wallet-standard/core` |

**Prefer `debridge-solana-bridge.mjs`** — the agent calls MCP to get the quote, then pipes the JSON to the script via stdin. This avoids passing large hex strings through shell variables.

## Prerequisites

- OWS wallet created (`ows wallet create`) with a funded Solana address
- deBridge MCP configured (see ../common/mcp-setup.md)
- OWS SDK installed for your environment (see Install table above)
- Solana RPC endpoint — set `SOLANA_RPC_URL` env var or use the default public RPC (see ../common/rpc-discovery.md)

## Step 1: Get a deBridge Quote

Call `mcp__debridge__create_tx` with Solana as the source chain (chainId `7565164`). The response `tx.data` is a hex-encoded Solana V0 versioned transaction with a placeholder blockhash (all zeros).

## Step 2: Sign and Broadcast

⚠️ CAUTION: This executes a real transaction.

### Node.js

```bash
node scripts/ows-solana-sign.mjs <tx_hex> <wallet_name>
```

Examples:

```bash
# Basic — sign and broadcast, print tx hash
node scripts/ows-solana-sign.mjs 01000000...abcdef agent-treasury

# Custom RPC + JSON output (for piping to monitoring)
node scripts/ows-solana-sign.mjs 01000000...abcdef agent-treasury \
  --rpc https://my-rpc.example.com --json

# Using SOLANA_RPC_URL env var
SOLANA_RPC_URL=https://my-rpc.example.com \
  node scripts/ows-solana-sign.mjs 01000000...abcdef agent-treasury
```

### What the Script Does

The script performs five steps (see source for detailed comments):

1. Parse the hex-encoded Solana V0 transaction from deBridge
2. Fetch a fresh blockhash from Solana RPC (`getLatestBlockhash`)
3. Insert the blockhash into the transaction at the correct offset
4. Sign the message bytes (offset 65+) via OWS SDK `signMessage(wallet, chain, message, undefined, "hex")` (Node.js) / `sign_message(wallet, chain, message, encoding="hex")` (Python)
5. Assemble the final transaction, convert to base64, and broadcast via `sendTransaction`

All steps run in a single invocation because blockhashes expire in ~60–90 seconds.

### Output

- Default: prints the transaction signature (hash) on success
- `--json`: prints the full Solana RPC response as JSON
- Exit code 1 on error (RPC failure, broadcast rejection)

## Example Prompts

Cross-chain from Solana:
- "Bridge 10 USDC from Solana to Arbitrum"
- "Swap 2 SOL from Solana to ETH on Ethereum"

Quote only:
- "Get a deBridge quote for 100 USDC from Solana to Base, don't execute yet"

## Security Notes

- OWS wallet keys are encrypted locally and decrypted only in-process during signing — never exposed in script output.
- Use `confirmed` commitment (not `finalized`) for the freshest blockhash.
- For testing, use small amounts to minimize risk from blockhash expiry or failed broadcasts.
- deBridge MCP is read-only by itself — it generates tx data but cannot sign or broadcast.
- Account for ~0.024 SOL in rent/fees on top of the bridge amount when funding the wallet.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `Cannot find module '@open-wallet-standard/core'` | Run `npm install @open-wallet-standard/core` |
| No `ows sign send-tx` for Solana | `signAndSend` is optional in the OWS spec and not available as a CLI command. Use the bundled scripts |
| `ows sign tx` produces invalid signature | Signs the exact bytes you pass without extracting Solana message portion. The scripts handle extraction automatically |
| Blockhash expired | All steps run in one invocation (~60–90s window). Re-run the script |
| Transaction simulation failed | Check wallet has enough SOL for rent/fees (~0.024 SOL) on top of the bridge amount |
| Signature verification failed | Ensure you're using the scripts (they insert the blockhash before signing, as required) |
