---
title: deBridge Signing with OWS (Open Wallet Standard)
impact: HIGH
impactDescription: "Local self-custody signing for deBridge transactions via OWS CLI"
tags: ows, signing, self-custody, local-key, evm, solana, tron, deBridge
---

# deBridge Signing with OWS

Local self-custody signing for deBridge transactions using OWS wallets. Keys stay encrypted on the user's machine and are decrypted only in-process during signing — never exposed.

OWS supports several chain families including

| Chain family | OWS signing | deBridge support | Notes |
|--------------|-------------|------------------|-------|
| EVM          | `ows sign tx` / `ows sign message` | All EVM chains (Ethereum, Arbitrum, Base, Polygon, etc.) | Standard EVM tx signing works directly |
| Solana       | `ows sign message --encoding hex` | Solana (chainId `7565164`) | Requires manual flow — see below |
| Tron         | `ows sign tx` / `ows sign message` | Tron (chainId `100000026`) | base58check addresses, 6-decimal TRX |

## Solana Manual Sign + Broadcast

The rest of this document covers the Solana-specific workaround. For EVM and Tron, `ows sign tx` works directly with the hex transaction data returned by `mcp__debridge__create_tx`.

The agent gets a quote from deBridge MCP, then signs locally and broadcasts via a single script — required because the OWS CLI lacks Solana-aware transaction handling.

```
User prompt → deBridge MCP (quote + tx data) → Script (blockhash → sign → broadcast) → On-chain
```

Works for Solana-to-EVM bridges, Solana-to-Solana swaps routed cross-chain, and any deBridge order with Solana as the source.

## Prerequisites

- Node.js 18+ (preferred) or Python 3 with `base58` (`pip3 install base58`)
- deBridge MCP configured (see ../common/mcp-setup.md)
- OWS wallet with a funded Solana wallet (bridge amount + ~0.024 SOL for rent/fees)
- Solana RPC endpoint — set `SOLANA_RPC_URL` env var or use the default public RPC (see ../common/rpc-discovery.md)

## Why a Manual Flow?

The OWS CLI does not have a dedicated `signAndSend` subcommand for Solana (`signAndSend` is an optional feature in the OWS spec and is not exposed as a CLI command). Additionally, `ows sign tx` signs whatever bytes you pass — it does not extract the Solana message portion automatically. For Solana V0 versioned transactions, the signature must cover only the message bytes (offset 65+), not the full serialized transaction including signature placeholders.

The solution: use `ows sign message --encoding hex` on the message bytes only (offset 65+), then broadcast the assembled transaction as base64 yourself. All steps must run in a single script because blockhashes expire in ~60–90 seconds.

## Step 1: Get a deBridge Quote

Call `mcp__debridge__create_tx` with Solana as the source chain (chainId `7565164`). The response `tx.data` is a hex-encoded Solana V0 versioned transaction with a placeholder blockhash (all zeros).

## Step 2: Sign and Broadcast

⚠️ CAUTION: This executes a real transaction.

Run everything in one script. The flow:

1. Fetch a fresh blockhash from Solana RPC
2. Insert the blockhash into the transaction (before signing — the signature covers the blockhash)
3. Sign the message bytes (offset 65+) with OWS
4. Insert the signature at bytes 1–64
5. Broadcast the full transaction as base64

### Option A: Node.js (preferred)

Zero external dependencies — uses only Node.js built-ins (`Buffer`, `fetch`, `child_process`) plus an inline base58 decoder.

```js
const { execSync } = require('child_process');

// -- CONFIG: set these from deBridge response --
const txHex = '<tx.data from deBridge, strip leading 0x>';
import { Connection } from '@solana/web3.js';
import bs58 from 'bs58';

const wallet = 'agent-treasury'; // OWS wallet name
const rpcUrl = process.env.SOLANA_RPC_URL || 'https://api.mainnet-beta.solana.com';
const connection = new Connection(rpcUrl, 'confirmed');

const tx = Buffer.from(txHex, 'hex');

// Parse V0 transaction:
// Byte 0: 01 (1 sig), Bytes 1-64: sig placeholder, Byte 65: 0x80 (V0),
// Bytes 66-68: header, Byte 69: num_account_keys
const numKeys = tx[69];
const bhOffset = 70 + numKeys * 32;

// Step 1: Fresh blockhash
const { blockhash } = await connection.getLatestBlockhash('confirmed');

// Step 2: Insert blockhash BEFORE signing
const bhBytes = bs58.decode(blockhash);
Buffer.from(bhBytes).copy(tx, bhOffset);

// Step 3: Sign message bytes (offset 65+) with OWS
const messageHex = tx.subarray(65).toString('hex');
const sigJson = execSync(
  `ows sign message --chain solana --wallet ${wallet} --encoding hex --message ${messageHex} --json`,
  { encoding: 'utf-8' }
);
const { signature } = JSON.parse(sigJson);

// Step 4: Insert signature at bytes 1-64
Buffer.from(signature, 'hex').copy(tx, 1);

// Step 5: Broadcast
const txSignature = await connection.sendRawTransaction(tx, {
  skipPreflight: false,
  preflightCommitment: 'confirmed',
});

console.log('Transaction sent:', txSignature);
```

Run with: `node --experimental-vm-modules` or wrap in an async IIFE.

### Option B: Python (fallback)

Requires `pip3 install base58`.

```python
import base58, base64, json, os, subprocess, urllib.request

tx_hex = "<tx.data from deBridge, strip leading 0x>"
wallet = "agent-treasury"
rpc_url = os.environ.get("SOLANA_RPC_URL", "https://api.mainnet-beta.solana.com")
tx = bytearray(bytes.fromhex(tx_hex))

num_keys = tx[69]
bh_offset = 70 + num_keys * 32

# Fresh blockhash
req = urllib.request.Request(rpc_url,
    data=json.dumps({"jsonrpc":"2.0","id":1,"method":"getLatestBlockhash",
        "params":[{"commitment":"confirmed"}]}).encode(),
    headers={"Content-Type":"application/json"})
bh_b58 = json.loads(urllib.request.urlopen(req).read())["result"]["value"]["blockhash"]
tx[bh_offset:bh_offset+32] = base58.b58decode(bh_b58)

# Sign message (offset 65+)
sig = json.loads(subprocess.run(
    ["ows","sign","message","--chain","solana","--wallet",wallet,
     "--encoding","hex","--message",tx[65:].hex(),"--json"],
    capture_output=True, text=True).stdout)["signature"]
tx[1:65] = bytes.fromhex(sig)

# Broadcast as base64
resp = urllib.request.urlopen(urllib.request.Request(rpc_url,
    data=json.dumps({"jsonrpc":"2.0","id":1,"method":"sendTransaction",
        "params":[base64.b64encode(bytes(tx)).decode(),
            {"encoding":"base64","skipPreflight":False,"preflightCommitment":"confirmed"}]}).encode(),
    headers={"Content-Type":"application/json"}))
print(json.loads(resp.read()))
```

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
| No `ows sign send-tx` for Solana | `signAndSend` is optional in the OWS spec and not available as a CLI command. Use the manual sign + broadcast script above |
| `ows sign tx` produces invalid signature | Signs the exact bytes you pass without extracting Solana message portion. Use `ows sign message --encoding hex` on offset 65+ only |
| Blockhash expired | All steps must run in one script. Blockhashes last ~60–90s. Re-run the full script |
| Transaction simulation failed | Check wallet has enough SOL for rent/fees (~0.024 SOL) on top of the bridge amount |
| Signature verification failed | Ensure blockhash is inserted BEFORE signing — the signature covers the blockhash |
