---
title: Signing with ethers.js or viem
impact: HIGH
impactDescription: "Primary signing method for Node.js CLI and headless agents"
tags: signing, ethers, viem, eip-712, private-key, typescript, javascript
---

# ethers.js / viem Signing

## Setup

```typescript
// ethers v6
import { ethers } from "ethers";

const provider = new ethers.JsonRpcProvider(process.env.RPC_URL);
const wallet = new ethers.Wallet(process.env.PRIVATE_KEY, provider);
```

```typescript
// viem
import { createWalletClient, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { mainnet } from "viem/chains";

const account = privateKeyToAccount(process.env.PRIVATE_KEY as `0x${string}`);
const client = createWalletClient({
  account,
  chain: mainnet, // replace with source chain
  transport: http(process.env.RPC_URL),
});
```

⚠️ CAUTION: Never log or expose `PRIVATE_KEY`. Read from environment only.

## Sign and Send Transaction

Given tx data from `mcp__debridge__create_tx`:

```typescript
// ethers v6
const txResponse = await wallet.sendTransaction({
  to: tx.to,
  data: tx.data,
  value: tx.value,
  chainId: tx.chainId,
  gasLimit: tx.gasLimit, // if provided
});
const receipt = await txResponse.wait(1); // wait 1 confirmation
console.log("tx hash:", receipt.hash);
```

```typescript
// viem
const hash = await client.sendTransaction({
  to: tx.to as `0x${string}`,
  data: tx.data as `0x${string}`,
  value: BigInt(tx.value || "0"),
  chain: sourceChain, // match source chain
});
```

## Sign EIP-712 Typed Data

For DLN order signatures:

```typescript
// ethers v6
const signature = await wallet.signTypedData(
  typedData.domain,
  typedData.types,
  typedData.message
);
```

```typescript
// viem
const signature = await client.signTypedData({
  domain: typedData.domain,
  types: typedData.types,
  primaryType: typedData.primaryType,
  message: typedData.message,
});
```

## Approval Transaction

If `create_tx` returns an `approveTx`, send it first:

```typescript
// ethers v6
const approveResponse = await wallet.sendTransaction({
  to: approveTx.to,
  data: approveTx.data,
  value: "0",
});
await approveResponse.wait(1); // must confirm before bridge tx
```

## Common Errors

| Error | Fix |
|-------|-----|
| `INSUFFICIENT_FUNDS` | Fund wallet with native token for gas |
| `NONCE_EXPIRED` | Pending tx — wait or use `wallet.getNonce("pending")` |
| `CALL_EXCEPTION` | Tx will revert — check approval, balance, and params |
