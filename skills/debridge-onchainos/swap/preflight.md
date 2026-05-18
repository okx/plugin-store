---
title: Preflight Checks
impact: HIGH
impactDescription: "Mandatory validation before any bridge or swap execution"
tags: preflight, balance, allowance, gas, slippage, validation
---

# Preflight Checks

Run all checks before signing any transaction. Stop if any check fails.

## 1. Source Token Balance

Verify the wallet holds enough of the source token.

### ERC-20 Token

```bash
# cast
cast call "$TOKEN_ADDRESS" "balanceOf(address)" "$WALLET_ADDRESS" --rpc-url "$RPC_URL"
```

```typescript
// ethers v6
const balance = await tokenContract.balanceOf(walletAddress);
// balance must be >= srcChainTokenInAmount (both in raw units)
```

If balance < required amount → **STOP**. Inform the user of the shortfall.

### Native Token (ETH, BNB, etc.)

```bash
cast balance "$WALLET_ADDRESS" --rpc-url "$RPC_URL"
```

For native token bridges, the balance must cover both the bridge amount AND gas.

## 2. Token Allowance (ERC-20 only)

Skip this check for native token transfers.

Check if the deBridge contract is approved to spend the token:

```bash
cast call "$TOKEN_ADDRESS" "allowance(address,address)" "$WALLET_ADDRESS" "$SPENDER_ADDRESS" --rpc-url "$RPC_URL"
```

The `$SPENDER_ADDRESS` is the `tx.to` field from the `create_tx` or `transaction_same_chain_swap` response.

If allowance < required amount:
- The response should include an `approveTx` object.
- Send the approval transaction first (see ../signing/SKILL.md).
- Wait for 1 confirmation before proceeding.

## 3. Gas Budget

Verify the wallet has enough native token to pay for gas:

```bash
cast balance "$WALLET_ADDRESS" --rpc-url "$RPC_URL"
```

Estimate gas cost:
- Approval tx: ~50,000 gas
- Bridge tx: ~200,000–500,000 gas (varies by route)
- Total native needed: `(approval_gas + bridge_gas) × gas_price`

If the bridge is for the native token, the balance must cover `bridge_amount + gas_cost`.

If insufficient gas → **STOP**. User needs to fund the wallet with native token.

## 4. Slippage

The `create_tx` call may accept slippage parameters. If the estimated output amount is significantly less than expected:

- Re-call `create_tx` to get a fresh quote (prices change).
- If slippage is too high, inform the user and ask whether to proceed.
- For automated agents, a default slippage tolerance of 1% (100 bps) is reasonable. Increase for volatile tokens or illiquid pairs.

## Summary

| Check       | Fails when                         | Action                          |
|-------------|------------------------------------|---------------------------------|
| Balance     | Token balance < bridge amount      | Stop, inform user               |
| Allowance   | Allowance < bridge amount          | Send approval tx, wait, retry   |
| Gas         | Native balance < estimated gas     | Stop, user must fund wallet     |
| Slippage    | Output significantly below expect  | Re-quote or increase tolerance  |

All checks pass → proceed to ../signing/SKILL.md.

## Bundled Scripts

The `../common/scripts/` directory has TypeScript helpers that automate these checks. All scripts auto-discover RPC endpoints from Chainlist.

| Script | Preflight use | Example |
|--------|--------------|---------|
| `balance.ts` | Check token or native balance | `npx tsx ../common/scripts/balance.ts 0xAddr 42161 --token 0xUSDC` |
| `allowance.ts` | Check ERC-20 allowance | `npx tsx ../common/scripts/allowance.ts 0xUSDC 0xOwner 0xSpender 42161 --check 1000000000` |
| `approve.ts` | Send approval tx | `npx tsx ../common/scripts/approve.ts 0xUSDC 0xSpender 42161 --amount 1000000000` |
