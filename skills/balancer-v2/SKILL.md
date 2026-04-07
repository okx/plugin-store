---
name: balancer-v2
version: "0.1.0"
description: "Balancer V2 DEX — swap tokens, query pools, add/remove liquidity on Arbitrum and Ethereum"
---

# Balancer V2 Skill

Balancer V2 is a DEX and AMM on Ethereum and Arbitrum featuring multi-token weighted pools, stable pools, and a single Vault contract as the unified entry point for all swaps and liquidity operations.

## Architecture

- All on-chain operations route through the **Vault** contract (`0xBA12222222228d8Ba445958a75a0704d566BF2C8`)
- Pool queries are served via **BalancerQueries** (`0xE39B5e3B6D74016b2F6A9673D7d7493B6DF549d5`)
- Pool discovery uses the Balancer Subgraph (GraphQL)
- Write ops → after user confirmation, submits via `onchainos wallet contract-call` with `--force`

## Commands

### pools — List Balancer V2 Pools

List the top pools by total liquidity on a given chain.

**Trigger phrases:**
- "show me Balancer pools on Arbitrum"
- "list top Balancer V2 pools"
- "what pools are on Balancer?"

**Usage:**
```
balancer-v2 pools [--chain <chain_id>] [--limit <n>]
```

**Parameters:**
- `--chain`: Chain ID (default: 42161 for Arbitrum; 1 for Ethereum)
- `--limit`: Number of pools to return (default: 20)

**Example:**
```
balancer-v2 pools --chain 42161 --limit 10
```

**Output:** JSON array of pools with id, address, poolType, totalLiquidity, swapFee, and token list.

---

### pool-info — Get Pool Details

Get detailed on-chain information for a specific Balancer V2 pool.

**Trigger phrases:**
- "show info for Balancer pool 0x6454..."
- "what tokens are in this Balancer pool?"
- "get pool details for Balancer pool ID ..."

**Usage:**
```
balancer-v2 pool-info --pool <pool_id> [--chain <chain_id>]
```

**Parameters:**
- `--pool`: Pool ID (bytes32, from Balancer UI or `pools` command)
- `--chain`: Chain ID (default: 42161)

**Example:**
```
balancer-v2 pool-info --pool 0x64541216bafffeec8ea535bb71fbc927831d0595000100000000000000000002 --chain 42161
```

**Output:** Pool address, specialization, swap fee %, total supply (BPT), token list with balances and weights.

---

### quote — Get Swap Quote

Get an estimated output amount for a swap using the on-chain BalancerQueries contract.

**Trigger phrases:**
- "quote swap 0.001 WETH for USDC on Balancer"
- "how much USDC will I get for 0.001 WETH on Balancer?"
- "Balancer quote: 1 USDT → WETH"

**Usage:**
```
balancer-v2 quote --from <token> --to <token> --amount <amount> --pool <pool_id> [--chain <chain_id>]
```

**Parameters:**
- `--from`: Input token symbol (WETH, USDC, USDT, WBTC) or address
- `--to`: Output token symbol or address
- `--amount`: Amount of input token (human-readable, e.g. 0.001)
- `--pool`: Pool ID to route through
- `--chain`: Chain ID (default: 42161)

**Example:**
```
balancer-v2 quote --from WETH --to USDC --amount 0.001 --pool 0x64541216bafffeec8ea535bb71fbc927831d0595000100000000000000000002 --chain 42161
```

**Output:** amountIn, amountOut (raw and human-readable).

---

### positions — View LP Positions

View the current wallet's BPT (Balancer Pool Token) holdings across known pools.

**Trigger phrases:**
- "show my Balancer LP positions"
- "what's my Balancer liquidity?"
- "list my Balancer V2 positions on Arbitrum"

**Usage:**
```
balancer-v2 positions [--chain <chain_id>] [--wallet <address>]
```

**Parameters:**
- `--chain`: Chain ID (default: 42161)
- `--wallet`: Wallet address (optional; defaults to connected onchainos wallet)

**Example:**
```
balancer-v2 positions --chain 42161
```

**Output:** JSON with pool_id, pool_address, bpt_balance, bpt_balance_raw per position.

---

### swap — Execute Token Swap

Swap tokens through a Balancer V2 pool via Vault.swap(). Performs ERC-20 approve (if needed) then calls Vault.swap with GIVEN_IN.

**Trigger phrases:**
- "swap 0.001 WETH for USDC on Balancer"
- "trade WETH to USDC on Balancer V2"
- "exchange USDT for WETH on Balancer Arbitrum"

**Usage:**
```
balancer-v2 swap --from <token> --to <token> --amount <amount> --pool <pool_id> [--slippage <pct>] [--chain <chain_id>] [--dry-run]
```

**Parameters:**
- `--from`: Input token symbol or address
- `--to`: Output token symbol or address
- `--amount`: Amount of input token (human-readable)
- `--pool`: Pool ID to swap through
- `--slippage`: Slippage tolerance in % (default: 0.5)
- `--chain`: Chain ID (default: 42161)
- `--dry-run`: Simulate without broadcasting

**Flow:**
1. Get quote via BalancerQueries.querySwap()
2. Run `--dry-run` to preview calldata
3. **Ask user to confirm** before submitting the transaction
4. If allowance insufficient: `onchainos wallet contract-call` (ERC-20 approve → Vault)
5. Execute: `onchainos wallet contract-call` → Vault.swap() with `--force`

**Example:**
```
balancer-v2 swap --from WETH --to USDC --amount 0.001 --pool 0x64541216bafffeec8ea535bb71fbc927831d0595000100000000000000000002 --chain 42161
```

**Output:** txHash, pool_id, asset_in, asset_out, amount_in, min_amount_out.

---

### join — Add Liquidity

Add liquidity to a Balancer V2 pool via Vault.joinPool(). Uses EXACT_TOKENS_IN_FOR_BPT_OUT join kind.

**Trigger phrases:**
- "add liquidity to Balancer pool 0x6454..."
- "provide liquidity on Balancer with 1 USDC"
- "join Balancer pool with tokens"

**Usage:**
```
balancer-v2 join --pool <pool_id> --amounts <a1,a2,a3> [--chain <chain_id>] [--dry-run]
```

**Parameters:**
- `--pool`: Pool ID
- `--amounts`: Comma-separated amounts per token in pool order (use 0 for tokens you don't want to provide)
- `--chain`: Chain ID (default: 42161)
- `--dry-run`: Simulate without broadcasting

**Flow:**
1. Query pool tokens via Vault.getPoolTokens()
2. Run `--dry-run` to preview calldata
3. **Ask user to confirm** before submitting
4. Approve each non-zero token: `onchainos wallet contract-call` (ERC-20 approve) with `--force`
5. Execute: `onchainos wallet contract-call` → Vault.joinPool() with `--force`

**Example:**
```
balancer-v2 join --pool 0x64541216bafffeec8ea535bb71fbc927831d0595000100000000000000000002 --amounts 0,0,1.0 --chain 42161
```

---

### exit — Remove Liquidity

Remove liquidity from a Balancer V2 pool via Vault.exitPool(). Burns BPT for proportional token output.

**Trigger phrases:**
- "remove liquidity from Balancer pool"
- "exit Balancer position, burn 0.001 BPT"
- "withdraw liquidity from Balancer"

**Usage:**
```
balancer-v2 exit --pool <pool_id> --bpt-amount <amount> [--chain <chain_id>] [--dry-run]
```

**Parameters:**
- `--pool`: Pool ID
- `--bpt-amount`: Amount of BPT to burn (human-readable)
- `--chain`: Chain ID (default: 42161)
- `--dry-run`: Simulate without broadcasting

**Flow:**
1. Query pool tokens via Vault.getPoolTokens()
2. Run `--dry-run` to preview calldata
3. **Ask user to confirm** before submitting the transaction
4. Execute: `onchainos wallet contract-call` → Vault.exitPool() with `--force`

**Example:**
```
balancer-v2 exit --pool 0x64541216bafffeec8ea535bb71fbc927831d0595000100000000000000000002 --bpt-amount 0.001 --chain 42161
```

## Supported Chains

| Chain | Chain ID | Notes |
|-------|----------|-------|
| Arbitrum | 42161 | Primary — WETH, USDC.e, USDT, WBTC |
| Ethereum | 1 | Secondary |

## Known Token Symbols

| Symbol | Arbitrum Address |
|--------|-----------------|
| WETH | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` |
| USDC / USDC.e | `0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8` |
| USDT | `0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9` |
| WBTC | `0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0F` |
| DAI | `0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1` |
