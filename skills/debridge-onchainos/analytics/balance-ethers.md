---
title: Balance Queries with ethers.js and viem
impact: HIGH
impactDescription: "Primary balance query method for Node.js CLI and headless agents"
tags: balance, ethers, viem, erc20, native, wallet, typescript, javascript
---

# ethers.js / viem Balance Queries

## Setup

### ethers v6

```typescript
import { ethers } from "ethers";

const provider = new ethers.JsonRpcProvider(process.env.RPC_URL);
// Derive address from private key (if no address provided)
const wallet = new ethers.Wallet(process.env.PRIVATE_KEY);
const address = wallet.address;
```

### viem

```typescript
import { createPublicClient, http } from "viem";
import { mainnet } from "viem/chains";
import { privateKeyToAccount } from "viem/accounts";

const client = createPublicClient({
  chain: mainnet, // replace with target chain
  transport: http(process.env.RPC_URL),
});
// Derive address from private key (if no address provided)
const account = privateKeyToAccount(process.env.PRIVATE_KEY as `0x${string}`);
const address = account.address;
```

## Native Token Balance

### ethers

```typescript
const balance = await provider.getBalance(address);
console.log(`${ethers.formatEther(balance)} ETH`);
```

### viem

```typescript
import { formatEther } from "viem";

const balance = await client.getBalance({ address });
console.log(`${formatEther(balance)} ETH`);
```

## ERC-20 Token Balance

### ethers

```typescript
const erc20Abi = ["function balanceOf(address) view returns (uint256)",
                  "function decimals() view returns (uint8)",
                  "function symbol() view returns (string)"];
const token = new ethers.Contract(tokenAddress, erc20Abi, provider);

const [balance, decimals, symbol] = await Promise.all([
  token.balanceOf(address),
  token.decimals(),
  token.symbol(),
]);
console.log(`${ethers.formatUnits(balance, decimals)} ${symbol}`);
```

### viem

```typescript
import { formatUnits } from "viem";
import { erc20Abi } from "viem";

const [balance, decimals, symbol] = await Promise.all([
  client.readContract({
    address: tokenAddress,
    abi: erc20Abi,
    functionName: "balanceOf",
    args: [address],
  }),
  client.readContract({
    address: tokenAddress,
    abi: erc20Abi,
    functionName: "decimals",
  }),
  client.readContract({
    address: tokenAddress,
    abi: erc20Abi,
    functionName: "symbol",
  }),
]);
console.log(`${formatUnits(balance, decimals)} ${symbol}`);
```

## Multi-Chain Balance Scan

**Recommended:** Use the bundled script instead of inline code:

```bash
node scripts/balance-evm.mjs 0x000A5539cD9505b44575c56f929C657c73899c30 --chains 1,42161,8453,10,137,56,43114,59144
```

The script accepts only a standard 0x EVM address (42 chars). Resolve wallet names first via WALLET_DISCOVERY.

For inline code, discover RPCs dynamically via `../common/scripts/rpc.mjs` (or `rpc.ts`):

### ethers

```typescript
import { ethers } from "ethers";
import { getRpc, getChainInfo } from "../common/scripts/rpc.mjs";

const chainIds = [1, 42161, 8453, 10, 137, 56, 43114, 59144];

async function getBalance(chainId: number, address: string) {
  const rpcUrl = await getRpc(chainId);
  const info = await getChainInfo(chainId);
  const provider = new ethers.JsonRpcProvider(rpcUrl, undefined, { staticNetwork: true });
  const balance = await Promise.race([
    provider.getBalance(address),
    new Promise<never>((_, rej) => setTimeout(() => rej(new Error("timeout")), 5000)),
  ]);
  return { name: info?.name, symbol: info?.nativeCurrency.symbol, balance: ethers.formatEther(balance) };
}

const results = await Promise.allSettled(chainIds.map(id => getBalance(id, address)));
for (const r of results) {
  if (r.status === "fulfilled") {
    const { name, symbol, balance } = r.value;
    console.log(`${name}: ${balance} ${symbol}`);
  }
}
```

Key details:
- RPCs are discovered from Chainlist with health checks — no hardcoded URLs.
- Use `staticNetwork: true` to skip the initial `eth_chainId` call — faster and avoids retry loops on slow RPCs.
- Always race with a timeout — public RPCs can hang indefinitely.
- Use `Promise.allSettled` (not `Promise.all`) so one failed chain does not abort the rest.

## ERC-20 Scan Across Chains

Check a specific token (e.g., USDC) on multiple chains. Token addresses come from `../common/chain-config.md`; RPCs are discovered dynamically:

```typescript
import { ethers } from "ethers";
import { getRpc } from "../common/scripts/rpc.mjs";

// USDC addresses from ../common/chain-config.md
const usdcByChain = [
  { chainId: 1,     name: "Ethereum", token: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", decimals: 6 },
  { chainId: 42161, name: "Arbitrum", token: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", decimals: 6 },
  { chainId: 8453,  name: "Base",     token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", decimals: 6 },
  { chainId: 137,   name: "Polygon",  token: "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359", decimals: 6 },
];

const abi = ["function balanceOf(address) view returns (uint256)"];

const results = await Promise.allSettled(
  usdcByChain.map(async (c) => {
    const rpc = await getRpc(c.chainId);
    const provider = new ethers.JsonRpcProvider(rpc, undefined, { staticNetwork: true });
    const contract = new ethers.Contract(c.token, abi, provider);
    const balance = await Promise.race([
      contract.balanceOf(address),
      new Promise<never>((_, rej) => setTimeout(() => rej(new Error("timeout")), 5000)),
    ]);
    return { name: c.name, balance: ethers.formatUnits(balance, c.decimals) };
  })
);
```

## Deriving Address from Private Key

When only `PRIVATE_KEY` is available and no address is known:

```typescript
// ethers
const address = new ethers.Wallet(process.env.PRIVATE_KEY).address;

// viem
import { privateKeyToAccount } from "viem/accounts";
const address = privateKeyToAccount(process.env.PRIVATE_KEY as `0x${string}`).address;
```

⚠️ CAUTION: Never log or expose `PRIVATE_KEY`. Derive the address, then discard the wallet object if signing is not needed.

## Common Errors

| Error | Fix |
|-------|-----|
| `staticNetwork` not recognized | ethers v6.7+ required — update: `npm install ethers@latest` |
| RPC timeout on `getBalance` | Use `Promise.race` with timeout; try next RPC from chainlist |
| `balanceOf` returns 0 unexpectedly | Verify token address matches the chain (USDC addresses differ per chain) |
| `CALL_EXCEPTION` on `decimals()` | Address is not an ERC-20 contract on this chain |
