---
name: curve
description: "Curve DEX plugin for swapping stablecoins and managing liquidity on Curve Finance. Trigger phrases: swap on Curve, Curve swap, add liquidity Curve, remove liquidity Curve, Curve pool APY, Curve pools, get Curve quote, Curve huan bi, zai Curve shang duihuan, Curve tianjia liudongxing, Curve cheyou liudongxing, chaxun Curve chizi."
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

## Scope

Do NOT use for: Uniswap swaps, Aave lending, general ERC-20 transfers, wallet balance queries unrelated to Curve LP positions, or any non-Curve DeFi protocol.
Do NOT use for: installing plugins from the plugin-store.
Do NOT use for: bridging assets between chains.

## Architecture

- Read ops (`get-pools`, `get-pool-info`, `get-balances`, `quote`) → direct `eth_call` via public RPC; no confirmation needed
- Write ops (`swap`, `add-liquidity`, `remove-liquidity`) → after user confirmation, submits via `onchainos wallet contract-call`

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview calldata and expected output
2. **Ask user to confirm** before executing on-chain
3. Execute only after explicit user approval
4. Report transaction hash and block explorer link

## Supported Chains

| Chain | ID | Router |
|-------|----|--------|
| Ethereum | 1 | CurveRouterNG 0x45312ea0... |
| Arbitrum | 42161 | CurveRouterNG 0x2191718C... |
| Base | 8453 | CurveRouterNG 0x4f37A9d1... |
| Polygon | 137 | CurveRouterNG 0x0DCDED35... |
| BSC | 56 | CurveRouterNG 0xA72C85C2... |

## Command Routing

| User Intent | Command |
|-------------|---------|
| "Show Curve pools on Ethereum" | `get-pools` |
| "What's the APY for Curve 3pool?" | `get-pool-info` |
| "How much LP do I have in Curve?" | `get-balances` |
| "Quote 1000 USDC → DAI on Curve" | `quote` |
| "Swap 1000 USDC for DAI on Curve" | `swap` |
| "Add liquidity to Curve 3pool" | `add-liquidity` |
| "Remove my Curve LP tokens" | `remove-liquidity` |

---

## get-pools — List Curve Pools

**Trigger phrases:** list Curve pools, show Curve pools, Curve pool list, Curve APY

**Usage:**
```
curve --chain <chain_id> get-pools [--registry main|crypto|factory|factory-crypto] [--limit 20]
```

**Parameters:**
- `--chain` — Chain ID (default: 1 = Ethereum)
- `--registry` — Registry type (omit to query all registries)
- `--limit` — Max pools to display sorted by TVL (default: 20)

**Expected output:**
```json
{
  "ok": true,
  "chain": "ethereum",
  "count": 20,
  "pools": [
    { "id": "3pool", "name": "Curve.fi DAI/USDC/USDT", "address": "0xbebc...", "tvl_usd": 123456789, "base_apy": "0.04%", "crv_apy": "1.25%" }
  ]
}
```

**No user confirmation required** — read-only query.

---

## get-pool-info — Pool Details

**Trigger phrases:** Curve pool info, Curve pool details, pool APY, Curve fee

**Usage:**
```
curve --chain <chain_id> get-pool-info --pool <pool_address>
```

**Parameters:**
- `--pool` — Pool contract address (from `get-pools` output)

**Expected output:** Pool name, coins, TVL, fee, virtual price.

**No user confirmation required** — read-only query.

---

## get-balances — LP Token Balances

**Trigger phrases:** my Curve LP, Curve liquidity position, how much LP do I have

**Usage:**
```
curve --chain <chain_id> get-balances [--wallet <address>]
```

**Parameters:**
- `--wallet` — Wallet address (default: onchainos active wallet)

**Expected output:** List of pools where wallet holds LP tokens, with raw balances.

**No user confirmation required** — read-only query.

---

## quote — Swap Quote

**Trigger phrases:** Curve quote, how much will I get on Curve, Curve price

**Usage:**
```
curve --chain <chain_id> quote --token-in <symbol|address> --token-out <symbol|address> --amount <minimal_units> [--slippage 0.005]
```

**Parameters:**
- `--token-in` — Input token symbol (USDC, DAI, USDT, WETH) or address
- `--token-out` — Output token symbol or address
- `--amount` — Input amount in minimal units (e.g. 1000000 = 1 USDC)
- `--slippage` — Slippage tolerance (default: 0.005 = 0.5%)

**Expected output:** Expected output amount, minimum with slippage, pool used, price impact.

**No user confirmation required** — read-only eth_call.

---

## swap — Execute Swap

**Trigger phrases:** swap on Curve, Curve swap, exchange on Curve, Curve DEX trade

**Usage:**
```
curve --chain <chain_id> [--dry-run] swap --token-in <symbol|address> --token-out <symbol|address> --amount <minimal_units> [--slippage 0.005] [--wallet <address>]
```

**Parameters:**
- `--token-in` — Input token symbol or address
- `--token-out` — Output token symbol or address
- `--amount` — Input amount in minimal units
- `--slippage` — Slippage tolerance (default: 0.005)
- `--wallet` — Sender address (default: onchainos active wallet)
- `--dry-run` — Preview without broadcasting

**Execution flow:**
1. Run `--dry-run` to preview expected output and calldata
2. **Ask user to confirm** the swap parameters and expected output
3. Check ERC-20 allowance; approve if needed
4. Execute via `onchainos wallet contract-call` with `--force`
5. Report `txHash` and block explorer link

**Example:**
```
curve --chain 1 swap --token-in USDC --token-out DAI --amount 1000000000 --slippage 0.005
```

---

## add-liquidity — Add Pool Liquidity

**Trigger phrases:** add liquidity Curve, deposit to Curve pool, provide liquidity Curve

**Usage:**
```
curve --chain <chain_id> [--dry-run] add-liquidity --pool <pool_address> --amounts <a1,a2,...> [--min-mint 0] [--wallet <address>]
```

**Parameters:**
- `--pool` — Pool contract address (obtain from `get-pools`)
- `--amounts` — Comma-separated token amounts in minimal units matching pool coin order (e.g. `"0,1000000,1000000"` for 3pool: DAI,USDC,USDT)
- `--min-mint` — Minimum LP tokens to accept (default: 0)
- `--wallet` — Sender address

**Execution flow:**
1. Run `--dry-run` to preview calldata
2. **Ask user to confirm** the amounts and pool address
3. Approve each non-zero token for the pool contract (checks allowance first)
4. Wait 5 seconds for approvals to confirm
5. Execute `add_liquidity` via `onchainos wallet contract-call` with `--force`
6. Report `txHash` and estimated LP tokens received

**Example — 3pool (DAI/USDC/USDT), supply 500 USDC + 500 USDT:**
```
curve --chain 1 add-liquidity --pool 0xbebc44782c7db0a1a60cb6fe97d0b483032ff1c7 --amounts "0,500000000,500000000"
```

---

## remove-liquidity — Remove Pool Liquidity

**Trigger phrases:** remove liquidity Curve, withdraw from Curve pool, redeem Curve LP

**Usage:**
```
curve --chain <chain_id> [--dry-run] remove-liquidity --pool <pool_address> [--lp-amount <raw>] [--coin-index <i>] [--min-amounts <a1,a2>] [--wallet <address>]
```

**Parameters:**
- `--pool` — Pool contract address
- `--lp-amount` — LP tokens to redeem (default: full wallet balance)
- `--coin-index` — Coin index for single-coin withdrawal (omit for proportional)
- `--min-amounts` — Minimum amounts to receive (default: 0)
- `--wallet` — Sender address

**Execution flow:**
1. Query LP token balance for the pool
2. If `--coin-index` provided: estimate single-coin output via `calc_withdraw_one_coin`
3. Run `--dry-run` to preview
4. **Ask user to confirm** before proceeding
5. Execute `remove_liquidity` or `remove_liquidity_one_coin` via `onchainos wallet contract-call` with `--force`
6. Report `txHash` and explorer link

**Example — remove all LP as USDC (coin index 1 in 3pool):**
```
curve --chain 1 remove-liquidity --pool 0xbebc44782c7db0a1a60cb6fe97d0b483032ff1c7 --coin-index 1 --min-amounts 0
```

**Example — proportional withdrawal from 2-pool:**
```
curve --chain 42161 remove-liquidity --pool <2pool_addr> --min-amounts "0,0"
```

---

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `CurveRouterNG not available on chain X` | Chain not supported | Use chain 1, 42161, 8453, 137, or 56 |
| `No Curve pool found containing both tokens` | Tokens not in same pool | Check `get-pools` output; may need multi-hop |
| `Quote returned 0` | Pool has insufficient liquidity | Try a different pool or smaller amount |
| `No LP token balance` | Wallet has no LP in that pool | Check `get-balances` first |
| `Cannot determine wallet address` | Not logged in to onchainos | Run `onchainos wallet login` |
| `txHash: pending` | Transaction not broadcast | `--force` flag is applied automatically for write ops |

## Security Notes

- Pool addresses are fetched from the official Curve API (`api.curve.finance`) only — never from user input
- ERC-20 allowance is checked before each approve to avoid duplicate transactions
- Price impact > 5% triggers a warning; handle in agent before calling `swap`
- Use `--dry-run` to preview all write operations before execution
