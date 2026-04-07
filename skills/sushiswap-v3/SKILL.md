---
name: sushiswap-v3
description: Swap tokens and manage concentrated liquidity positions on SushiSwap V3 CLMM across 38+ EVM chains
---

# SushiSwap V3

SushiSwap V3 is a Uniswap V3-style Concentrated Liquidity Market Maker (CLMM) deployed on 38+ EVM chains. LP positions are ERC-721 NFTs with tick-ranged concentrated liquidity. This plugin supports swapping, quoting, adding/removing liquidity, collecting fees, and querying positions and pools.

**Primary Chain**: Base (chain ID 8453)  
**Supported Chains**: Ethereum (1), Base (8453), Arbitrum (42161), BSC (56), Polygon (137), Optimism (10), Avalanche (43114)

**Architecture**: Read-only operations (quote, get-positions, get-pools) use direct `eth_call` via JSON-RPC. Write operations go through `onchainos wallet contract-call` after user confirmation.

**Contract Addresses** (same across all chains — deterministic CREATE2):
- Factory: `0xc35DADB65012eC5796536bD9864eD8773aBc74C4`
- SwapRouter: `0xFB7eF66a7e61224DD6FcD0D7d9C3be5C8B049b9f`
- QuoterV2: `0xb1E835Dc2785b52265711e17fCCb0fd018226a6e`
- NonfungiblePositionManager: `0x80C7DD17B01855a6D2347444a0FCC36136a314de`

**Fee Tiers**: 100 (0.01%), 500 (0.05%), 3000 (0.3%), 10000 (1%)

---

## Pre-flight Checks

```bash
# Ensure onchainos CLI is installed and wallet is configured
onchainos wallet balance --chain 8453

# Ensure the sushiswap-v3 binary is in your PATH
sushiswap-v3 --version
```

---

## Commands

### 1. `quote` — Get a Swap Quote

Queries QuoterV2 via `eth_call` — no transaction, no gas.

```bash
sushiswap-v3 quote \
  --token-in WETH \
  --token-out USDC \
  --amount-in 1000000000000000 \
  --chain 8453
```

**With specific fee tier:**
```bash
sushiswap-v3 quote \
  --token-in WETH \
  --token-out USDC \
  --amount-in 1000000000000000 \
  --fee 500 \
  --chain 8453
```

**Output:**
```json
{"ok":true,"tokenIn":"0x4200000000000000000000000000000000000006","tokenOut":"0x833589fcd6edb6e08f4c7c32d4f71b54bda02913","amountIn":1000000000000000,"bestFee":500,"amountOut":2052494}
```

**Notes:**
- Checks Factory for pool existence before calling QuoterV2
- If no fee is specified, queries all tiers (100, 500, 3000, 10000) and returns the best
- Use symbols: WETH/ETH, USDC, USDT, SUSHI; or hex addresses

---

### 2. `swap` — Swap Tokens

Executes `exactInputSingle` on the SushiSwap V3 SwapRouter. Quotes first, then proceeds.

```bash
sushiswap-v3 swap \
  --token-in WETH \
  --token-out USDC \
  --amount-in 50000000000000 \
  --slippage 0.5 \
  --chain 8453
```

**Dry run (no broadcast):**
```bash
sushiswap-v3 swap \
  --token-in WETH \
  --token-out USDC \
  --amount-in 50000000000000 \
  --dry-run \
  --chain 8453
```

**Output:**
```json
{"ok":true,"txHash":"0xabc...","tokenIn":"0x4200...","tokenOut":"0x8335...","amountIn":50000000000000,"fee":500,"amountOutMin":100123}
```

**Flow:**
1. QuoterV2 `eth_call` to find best fee tier and expected output
2. **Ask user to confirm** the quote (token amounts, fee tier, slippage)
3. Check ERC-20 allowance; if insufficient, submit `approve` via `wallet contract-call --force`
4. Wait 3 seconds for approve nonce to clear
5. Submit `exactInputSingle` via `wallet contract-call --force`

**Important Notes:**
- Requires `--force` flag (handled internally) for EVM DEX transactions
- Recipient is always the connected wallet address
- Use `--dry-run` to verify calldata selector without spending gas

---

### 3. `get-pools` — Query Pool Addresses

Queries the Factory for pool addresses by token pair.

```bash
sushiswap-v3 get-pools \
  --token0 WETH \
  --token1 USDC \
  --chain 8453
```

**Query specific fee:**
```bash
sushiswap-v3 get-pools --token0 WETH --token1 USDC --fee 500 --chain 8453
```

**Output:**
```json
{"ok":true,"token0":"0x4200...","token1":"0x8335...","chain":8453,"pools":[{"fee":100,"feePct":"0.01%","address":"0x...","deployed":false},{"fee":500,"feePct":"0.05%","address":"0xabc...","deployed":true},...]}
```

---

### 4. `get-positions` — List LP Positions

Lists all V3 LP positions (NFTs) owned by a wallet.

```bash
# Use connected wallet
sushiswap-v3 get-positions --chain 8453

# Query specific address
sushiswap-v3 get-positions \
  --owner 0x87fb0647faabea33113eaf1d80d67acb1c491b90 \
  --chain 8453
```

**Output:**
```json
{"ok":true,"owner":"0x87fb...","chain":8453,"positions":[{"tokenId":12345,"token0":"0x4200...","token1":"0x8335...","fee":500,"tickLower":-887270,"tickUpper":887270,"liquidity":"1234567890","tokensOwed0":"0","tokensOwed1":"0"}]}
```

---

### 5. `add-liquidity` — Add Concentrated Liquidity

Mints a new LP position NFT via NonfungiblePositionManager.

```bash
sushiswap-v3 add-liquidity \
  --token0 WETH \
  --token1 USDC \
  --fee 500 \
  --tick-lower -887270 \
  --tick-upper 887270 \
  --amount0-desired 1000000000000000 \
  --amount1-desired 2052494 \
  --chain 8453
```

**Dry run:**
```bash
sushiswap-v3 add-liquidity \
  --token0 WETH --token1 USDC --fee 500 \
  --tick-lower -887270 --tick-upper 887270 \
  --amount0-desired 1000000000000000 --amount1-desired 2052494 \
  --dry-run --chain 8453
```

**Output:**
```json
{"ok":true,"txHash":"0xabc...","token0":"0x4200...","token1":"0x8335...","fee":500,"tickLower":-887270,"tickUpper":887270}
```

**Flow:**
1. Verifies pool exists via Factory
2. **Ask user to confirm** token amounts and tick range
3. Approves token0 and token1 for NonfungiblePositionManager if needed
4. Submits `mint` via `wallet contract-call --force`

**Tick Guidelines:**
- Full range: `--tick-lower -887270 --tick-upper 887270`
- WETH/USDC ±1% range: roughly `--tick-lower -200 --tick-upper 200`
- Ticks must be divisible by the fee tier's tick spacing (500 fee → spacing 10)

---

### 6. `remove-liquidity` — Remove Liquidity

Calls `decreaseLiquidity` + `collect` on NonfungiblePositionManager.

```bash
sushiswap-v3 remove-liquidity \
  --token-id 12345 \
  --chain 8453
```

**Remove partial liquidity:**
```bash
sushiswap-v3 remove-liquidity \
  --token-id 12345 \
  --liquidity 500000000000000000 \
  --chain 8453
```

**Remove all and burn the NFT:**
```bash
sushiswap-v3 remove-liquidity \
  --token-id 12345 \
  --burn \
  --chain 8453
```

**Output:**
```json
{"ok":true,"tokenId":12345,"decreaseTx":"0xabc...","collectTx":"0xdef...","burnTx":""}
```

**Flow:**
1. Fetches current position via `positions()` call
2. **Ask user to confirm** the amount of liquidity to remove
3. Submits `decreaseLiquidity` via `wallet contract-call --force`
4. Waits 5 seconds for nonce to clear
5. Submits `collect` to sweep all tokens to wallet
6. If `--burn`: submits `burn` to destroy the empty NFT

---

### 7. `collect-fees` — Collect Accumulated Fees

Collects trading fees owed to a position.

```bash
sushiswap-v3 collect-fees \
  --token-id 12345 \
  --chain 8453
```

**Dry run:**
```bash
sushiswap-v3 collect-fees --token-id 12345 --dry-run --chain 8453
```

**Output:**
```json
{"ok":true,"txHash":"0xabc...","tokenId":12345,"recipient":"0x87fb..."}
```

**Flow:**
1. Fetches position to display `tokensOwed0` and `tokensOwed1`
2. If no fees owed, returns early (no transaction)
3. **Ask user to confirm** the fee amounts before collecting
4. Submits `collect` via `wallet contract-call --force` with `amount0Max = amount1Max = uint128::MAX`

---

## Token Address Reference (Base Chain 8453)

| Symbol | Address |
|--------|---------|
| WETH | `0x4200000000000000000000000000000000000006` |
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` |
| cbBTC | `0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf` |
| DAI | `0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb` |

---

## Error Reference

| Error | Cause | Fix |
|-------|-------|-----|
| `No valid pool or quote found` | Pool not deployed or no liquidity | Check with `get-pools`, try different fee tier |
| `Pool does not exist` | add-liquidity to non-existent pool | Deploy pool first or use existing fee tier |
| `Could not resolve wallet address` | onchainos wallet not configured | Run `onchainos wallet balance` to verify wallet |
| `eth_call error` | RPC error or wrong contract address | Check chain ID and token addresses |
