---
title: Browser Wallet Signing (MetaMask / EIP-1193)
impact: HIGH
impactDescription: "Required for browser-based agent environments with injected wallet"
tags: signing, metamask, browser, eip-1193, eip-712, injected-provider
---

# Browser Wallet Signing

## Prerequisites

Verify injected provider:
```javascript
if (typeof window.ethereum === "undefined") {
  throw new Error("No browser wallet detected. Install MetaMask or another EIP-1193 wallet.");
}
```

## Connect Wallet

```javascript
const accounts = await window.ethereum.request({
  method: "eth_requestAccounts",
});
const walletAddress = accounts[0];
```

## Switch Chain

Ensure the wallet is on the correct source chain before signing:

```javascript
await window.ethereum.request({
  method: "wallet_switchEthereumChain",
  params: [{ chainId: "0x" + sourceChainId.toString(16) }],
});
```

## Sign and Send Transaction

Given tx data from `mcp__debridge__create_tx`:

```javascript
const txHash = await window.ethereum.request({
  method: "eth_sendTransaction",
  params: [{
    from: walletAddress,
    to: tx.to,
    data: tx.data,
    value: tx.value ? "0x" + BigInt(tx.value).toString(16) : "0x0",
  }],
});
```

The wallet will prompt the user for confirmation. Wait for the user to approve.

## Sign EIP-712 Typed Data

For DLN order signatures:

```javascript
const signature = await window.ethereum.request({
  method: "eth_signTypedData_v4",
  params: [walletAddress, JSON.stringify(typedData)],
});
```

The `typedData` object must include `domain`, `types`, `primaryType`, and `message` fields.

## Approval Transaction

Send token approval before the bridge tx:

```javascript
const approveTxHash = await window.ethereum.request({
  method: "eth_sendTransaction",
  params: [{
    from: walletAddress,
    to: approveTxData.to,
    data: approveTxData.data,
    value: "0x0",
  }],
});
// Wait for confirmation before proceeding to bridge tx
```

## Common Errors

| Error | Fix |
|-------|-----|
| User rejected request | User declined in wallet popup — retry or explain why approval is needed |
| Chain mismatch | Call `wallet_switchEthereumChain` before signing |
| `eth_signTypedData_v4` not supported | Wallet is outdated — update MetaMask or use `eth_signTypedData_v3` |
