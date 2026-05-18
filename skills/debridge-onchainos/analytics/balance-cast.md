---
title: Balance Queries with Foundry Cast
impact: HIGH
impactDescription: "Balance query method for developers with Foundry toolchain"
tags: balance, foundry, cast, cli, erc20, native, wallet
---

# Foundry Cast Balance Queries

## Prerequisites

```bash
which cast || echo "Install Foundry: curl -L https://foundry.paradigm.xyz | bash && foundryup"
```

## Deriving Address from Private Key

```bash
ADDRESS=$(cast wallet address --private-key "$PRIVATE_KEY")
echo "$ADDRESS"
```

## Native Token Balance

### Single Chain

```bash
# Human-readable (ether units)
cast balance "$ADDRESS" --rpc-url "$RPC_URL" --ether

# Raw wei
cast balance "$ADDRESS" --rpc-url "$RPC_URL"
```

### Multi-Chain Scan

**Recommended:** Use the bundled script instead:

```bash
node scripts/balance-evm.mjs <address_or_wallet> --chains 1,42161,8453,10,137,56,43114,59144
```

For cast-based multi-chain scanning, discover RPCs dynamically:

```bash
#!/usr/bin/env bash
ADDRESS=$(cast wallet address --private-key "$PRIVATE_KEY")
CHAIN_IDS=(1 42161 8453 10 137 56 43114 59144)

for id in "${CHAIN_IDS[@]}"; do
  RPC=$(node ../common/scripts/rpc.mjs "$id")
  bal=$(cast balance "$ADDRESS" --rpc-url "$RPC" --ether 2>/dev/null) || bal="error"
  printf "chain %s: %s\n" "$id" "$bal"
done
```

## ERC-20 Token Balance

### Query Balance

```bash
# Raw units
cast call "$TOKEN_ADDRESS" "balanceOf(address)(uint256)" "$ADDRESS" --rpc-url "$RPC_URL"

# Get decimals to convert to human-readable
DECIMALS=$(cast call "$TOKEN_ADDRESS" "decimals()(uint8)" --rpc-url "$RPC_URL")
RAW=$(cast call "$TOKEN_ADDRESS" "balanceOf(address)(uint256)" "$ADDRESS" --rpc-url "$RPC_URL")

# Convert with cast
HUMAN=$(cast from-wei "$RAW" "$DECIMALS")
SYMBOL=$(cast call "$TOKEN_ADDRESS" "symbol()(string)" --rpc-url "$RPC_URL")
echo "$HUMAN $SYMBOL"
```

### USDC Across Chains

```bash
ADDRESS=$(cast wallet address --private-key "$PRIVATE_KEY")

# USDC addresses from ../common/chain-config.md
check_usdc() {
  local chain="$1" rpc="$2" token="$3"
  local raw
  raw=$(cast call "$token" "balanceOf(address)(uint256)" "$ADDRESS" --rpc-url "$rpc" 2>/dev/null) || { echo "$chain: error"; return; }
  local human
  human=$(cast from-wei "$raw" 6)
  printf "%-12s %s USDC\n" "$chain" "$human"
}

check_usdc "Ethereum" "$(node ../common/scripts/rpc.mjs 1)"     "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
check_usdc "Arbitrum" "$(node ../common/scripts/rpc.mjs 42161)" "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"
check_usdc "Base"     "$(node ../common/scripts/rpc.mjs 8453)"  "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
check_usdc "Polygon"  "$(node ../common/scripts/rpc.mjs 137)"   "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"
```

## Token Allowance Check

Useful before bridge/swap operations:

```bash
cast call "$TOKEN_ADDRESS" \
  "allowance(address,address)(uint256)" "$OWNER" "$SPENDER" \
  --rpc-url "$RPC_URL"
```

## Common Errors

| Error | Fix |
|-------|-----|
| `cast: command not found` | Install Foundry: `curl -L https://foundry.paradigm.xyz \| bash && foundryup` |
| Connection refused | RPC endpoint down — try another from chainlist |
| `from-wei` wrong result | Verify decimals — USDC is 6, not 18 |
| Empty response from `cast call` | Address is not a contract on this chain — check token address |
