---
title: Signing with Foundry Cast
impact: HIGH
impactDescription: "Primary signing method for developers with Foundry toolchain"
tags: signing, foundry, cast, cli, eip-712, private-key
---

# Foundry Cast Signing

## Prerequisites

```bash
which cast || echo "Install Foundry: curl -L https://foundry.paradigm.xyz | bash && foundryup"
```

## Sign and Send Transaction

Given tx data from `mcp__debridge__create_tx`:

### With private key from environment

```bash
cast send "$TX_TO" --data "$TX_DATA" \
  --value "$TX_VALUE" \
  --rpc-url "$RPC_URL" \
  --private-key "$PRIVATE_KEY" \
  --chain-id "$CHAIN_ID"
```

### With Foundry keystore

```bash
cast send "$TX_TO" --data "$TX_DATA" \
  --value "$TX_VALUE" \
  --rpc-url "$RPC_URL" \
  --account my-account \
  --chain-id "$CHAIN_ID"
```

Keystore accounts are stored in `~/.foundry/keystores/`. Create one:
```bash
cast wallet import my-account --interactive
```

⚠️ CAUTION: Prefer keystore over `--private-key` flag. The flag may leak the key to shell history.

## Sign EIP-712 Typed Data

```bash
cast wallet sign-auth \
  --private-key "$PRIVATE_KEY" \
  --rpc-url "$RPC_URL" \
  "$TYPED_DATA_JSON"
```

For complex typed data, write the JSON to a temp file:
```bash
echo "$TYPED_DATA_JSON" > /tmp/typed-data.json
cast wallet sign --data --from "$WALLET_ADDRESS" /tmp/typed-data.json
rm /tmp/typed-data.json
```

## Approval Transaction

Send token approval before the bridge tx:

```bash
# Approve max allowance for the spender
cast send "$TOKEN_ADDRESS" \
  "approve(address,uint256)" "$SPENDER_ADDRESS" "$(cast max-uint)" \
  --rpc-url "$RPC_URL" \
  --private-key "$PRIVATE_KEY"
```

Wait for confirmation before sending the bridge tx:
```bash
cast receipt "$APPROVAL_TX_HASH" --rpc-url "$RPC_URL"
```

## Useful Cast Commands

| Command | Purpose |
|---------|---------|
| `cast balance $ADDR --rpc-url $RPC` | Check native token balance |
| `cast call $TOKEN "balanceOf(address)" $ADDR --rpc-url $RPC` | Check token balance |
| `cast call $TOKEN "allowance(address,address)" $OWNER $SPENDER --rpc-url $RPC` | Check allowance |
| `cast chain-id --rpc-url $RPC` | Verify chain ID |
| `cast tx $HASH --rpc-url $RPC` | Get transaction details |

## Common Errors

| Error | Fix |
|-------|-----|
| `insufficient funds` | Fund wallet with native token for gas |
| `nonce too low` | Wait for pending tx: `cast nonce $ADDR --rpc-url $RPC` |
| `unknown account` | Keystore not found — check `ls ~/.foundry/keystores/` |
