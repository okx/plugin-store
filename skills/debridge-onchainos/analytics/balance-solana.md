---
title: Balance Queries on Solana
impact: HIGH
impactDescription: "Balance query method for Solana wallets using bundled script or @solana/web3.js"
tags: balance, solana, spl-token, native, wallet, rpc
---

# Solana Balance Queries

## RPC Endpoint

RPCs are resolved dynamically via `../common/scripts/rpc.mjs` (Solana chain ID `7565164`). Override order:
1. `$SOLANA_RPC_URL` environment variable
2. `$RPC_URL_7565164` environment variable
3. Public fallback

## Bundled Script (Recommended)

The fastest path — handles SOL balance and optional SPL token listing:

```bash
node scripts/balance-solana.mjs <base58_address> [--tokens] [--json]
```

The script accepts only a standard Solana base58 address (32-44 chars). Resolve wallet names first via WALLET_DISCOVERY (e.g., `ows wallet list` → extract the `solana:` address).

Examples:

```bash
# SOL balance for a resolved address
node scripts/balance-solana.mjs B7Z1whe4TX3tVXwb93Nsd9U4f4QZfnuzm5DyUnKxVSUr

# SOL + all SPL tokens
node scripts/balance-solana.mjs B7Z1whe4TX3tVXwb93Nsd9U4f4QZfnuzm5DyUnKxVSUr --tokens

# JSON output for piping
node scripts/balance-solana.mjs B7Z1whe4TX3tVXwb93Nsd9U4f4QZfnuzm5DyUnKxVSUr --tokens --json
```

## Native SOL Balance

### @solana/web3.js

```typescript
import { Connection, PublicKey, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { getRpc } from "../common/scripts/rpc.mjs";

const rpcUrl = await getRpc(7565164);
const connection = new Connection(rpcUrl);
const pubkey = new PublicKey(address);

const lamports = await connection.getBalance(pubkey);
console.log(`${lamports / LAMPORTS_PER_SOL} SOL`);
```

## SPL Token Balance

### @solana/web3.js + @solana/spl-token

```typescript
import { Connection, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddress, getAccount } from "@solana/spl-token";
import { getRpc } from "../common/scripts/rpc.mjs";

const rpcUrl = await getRpc(7565164);
const connection = new Connection(rpcUrl);
const owner = new PublicKey(address);
const mint = new PublicKey(tokenMintAddress);

const ata = await getAssociatedTokenAddress(mint, owner);
const account = await getAccount(connection, ata);
console.log(`Balance: ${account.amount.toString()} (raw)`);
```

## All SPL Token Balances

Use the bundled script: `node scripts/balance-solana.mjs <base58_address> --tokens`

## Common Errors

| Error | Fix |
|-------|-----|
| `Invalid param: could not find account` | Token account does not exist — wallet holds no balance of that token |
| RPC 429 rate limit | Use a dedicated RPC provider or add delay between calls |
| `FetchError` / timeout | Public RPC is congested — set `$SOLANA_RPC_URL` to a private endpoint |
| Wrong balance (too large/small) | SOL uses 9 decimals (1 SOL = 1e9 lamports); SPL tokens vary — check the mint's decimals |
