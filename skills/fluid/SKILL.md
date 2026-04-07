---
name: fluid
description: "Fluid Protocol — DEX + Lending by Instadapp. Supply/earn yield via ERC-4626 fTokens (fUSDC, fWETH), swap via Fluid AMM DEX, view positions. Trigger phrases: supply to fluid, deposit fUSDC, earn yield on fluid, fluid fToken, swap on fluid dex, fluid positions, fluid markets, fluid supply rates, fluid withdraw, withdraw from fluid, fluid protocol, instadapp fluid, 在Fluid存款, Fluid借贷, Fluid兑换, Fluid收益, Fluid仓位, Fluid流动性"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Fluid Protocol Skill

## Overview

Fluid is a combined DEX + Lending protocol by Instadapp with two main systems:

- **Fluid Lending** — ERC-4626 fToken contracts (fUSDC, fWETH, fGHO, fEURC). Users deposit assets and earn yield. No collateral required for lending.
- **Fluid DEX** — Novel concentrated AMM. Swap between paired tokens (EURC/USDC, wstETH/ETH, weETH/ETH, etc.)
- **Fluid Vault** — Collateral-based borrowing system (dry-run only due to liquidation risk)

**Supported chains:**

| Chain | Chain ID |
|-------|----------|
| Base (default) | 8453 |
| Ethereum Mainnet | 1 |
| Arbitrum | 42161 |

**Architecture:**
- Write operations (supply, withdraw, swap) → **ask user to confirm** before submitting via `onchainos wallet contract-call`
- Read operations (markets, positions, quote) → direct on-chain eth_call to resolver contracts; no confirmation needed
- Borrow/repay → dry-run only due to liquidation risk

---

## Pre-flight Checks

Before executing any command, verify:

1. **Binary installed**: `fluid --version` — if not found, instruct user to install the plugin
2. **Wallet connected**: `onchainos wallet status` — confirm logged in and active address is set

If wallet not connected:
```
Please connect your wallet first: run `onchainos wallet login`
```

---

## Command Routing Table

| User Intent | Command |
|-------------|---------|
| List fToken lending markets | `fluid markets` |
| Filter markets by asset | `fluid markets --asset USDC` |
| View my lending positions | `fluid positions` |
| Supply to fToken | `fluid supply --ftoken fUSDC --amount <n>` |
| Withdraw from fToken | `fluid withdraw --ftoken fUSDC --amount <n>` |
| Withdraw all from fToken | `fluid withdraw --ftoken fUSDC --all` |
| Swap on Fluid DEX | `fluid swap --token-in EURC --token-out USDC --amount-in <n>` |
| Get swap quote | `fluid quote --token-in EURC --token-out USDC --amount-in <n>` |
| Borrow (dry-run only) | `fluid --dry-run borrow --vault <addr> --amount <n>` |
| Repay (dry-run only) | `fluid --dry-run repay --vault <addr> --amount <n>` |

**Global flags:**
- `--chain <CHAIN_ID>` — 8453 (Base, default), 1 (Ethereum), 42161 (Arbitrum)
- `--from <ADDRESS>` — wallet address (defaults to active onchainos wallet)
- `--dry-run` — simulate without broadcasting

---

## Execution Flow for Write Operations

For all write operations (supply, withdraw, swap):

1. Run with `--dry-run` first to preview calldata
2. **Ask user to confirm** before executing on-chain
3. Execute only after receiving explicit user approval
4. Report transaction hash(es) and outcome

---

## Commands

### markets — List Fluid fToken lending markets

**Trigger phrases:** "fluid markets", "fluid supply rates", "fluid fTokens", "fluid yield", "Fluid利率", "Fluid市场"

**Usage:**
```bash
# List all fToken markets
fluid --chain 8453 markets

# Filter by asset
fluid --chain 8453 markets --asset USDC
fluid --chain 1 markets --asset WETH
```

**What it does:**
- Calls `LendingResolver.getFTokensEntireData()` on-chain
- Returns fToken address, underlying asset, supply rate
- Read-only — no confirmation needed

**Expected output:**
```json
{
  "ok": true,
  "chain": "Base",
  "chainId": 8453,
  "marketCount": 4,
  "markets": [
    {
      "name": "Fluid USDC",
      "symbol": "fUSDC",
      "fTokenAddress": "0xf42f5795...",
      "underlying": "USDC",
      "supplyInstruction": "fluid --chain 8453 supply --ftoken fUSDC --amount <n>"
    }
  ]
}
```

---

### positions — View your lending positions

**Trigger phrases:** "my fluid positions", "fluid portfolio", "fluid balance", "我的Fluid仓位", "Fluid持仓"

**Usage:**
```bash
fluid --chain 8453 positions
fluid --chain 8453 positions --from 0xYourAddress
fluid --chain 1 positions
```

**What it does:**
- Calls `LendingResolver.getUserPositions(user)` and checks `balanceOf` + `convertToAssets` per fToken
- Returns fToken shares and underlying asset value per position
- Read-only — no confirmation needed

**Expected output:**
```json
{
  "ok": true,
  "user": "0xYourAddress",
  "chain": "Base",
  "positions": [
    {
      "fToken": "0xf42f5795...",
      "symbol": "fUSDC",
      "fTokenShares": "9950000",
      "underlyingAssets": "10.05",
      "decimals": 6
    }
  ]
}
```

---

### supply — Supply to Fluid fToken (ERC-4626 deposit)

**Trigger phrases:** "supply to fluid", "deposit to fUSDC", "earn yield on fluid", "fluid deposit", "在Fluid存款", "Fluid存入"

**IMPORTANT:** Always run with `--dry-run` first, then ask user to confirm before proceeding.

**Usage:**
```bash
# Dry-run first
fluid --chain 8453 --dry-run supply --ftoken fUSDC --amount 10

# After user confirmation:
fluid --chain 8453 supply --ftoken fUSDC --amount 10

# Supply WETH
fluid --chain 8453 supply --ftoken fWETH --amount 0.001
```

**Key parameters:**
- `--ftoken` — fToken symbol (fUSDC, fWETH, fGHO, fEURC) or fToken contract address
- `--amount` — human-readable amount of **underlying** asset (e.g. 10 for 10 USDC)

**What it does:**
1. Resolves fToken address and underlying decimals
2. Step 1: Approves fToken to spend underlying asset — after user confirmation, submits via `onchainos wallet contract-call`
3. Step 2: Calls `deposit(assets, receiver)` (ERC-4626) — after user confirmation, submits via `onchainos wallet contract-call`

**Expected output:**
```json
{
  "ok": true,
  "operation": "supply",
  "fToken": "0xf42f5795D9ac7e9D757dB633D693cD548Cfd9169",
  "underlying": "USDC",
  "amount": "10",
  "approveTxHash": "0xabc...",
  "supplyTxHash": "0xdef..."
}
```

---

### withdraw — Withdraw from Fluid fToken

**Trigger phrases:** "withdraw from fluid", "redeem fUSDC", "take out from fluid", "从Fluid提款", "Fluid提现"

**IMPORTANT:** Always run with `--dry-run` first, then ask user to confirm before proceeding.

**Usage:**
```bash
# Partial withdrawal — dry-run first
fluid --chain 8453 --dry-run withdraw --ftoken fUSDC --amount 5

# After user confirmation:
fluid --chain 8453 withdraw --ftoken fUSDC --amount 5

# Full withdrawal — redeem all shares
fluid --chain 8453 withdraw --ftoken fUSDC --all
```

**Key parameters:**
- `--ftoken` — fToken symbol or address
- `--amount` — partial withdrawal amount in underlying token units (mutually exclusive with `--all`)
- `--all` — redeem entire fToken share balance

**Notes:**
- Partial withdrawal calls `withdraw(assets, receiver, owner)` (ERC-4626 selector `0xb460af94`)
- Full withdrawal calls `redeem(shares, receiver, owner)` (ERC-4626 selector `0xba087652`)
- After user confirmation, submits via `onchainos wallet contract-call`

**Expected output:**
```json
{
  "ok": true,
  "operation": "withdraw",
  "fToken": "0xf42f5795...",
  "amount": "5",
  "txHash": "0xabc..."
}
```

---

### swap — Swap via Fluid DEX

**Trigger phrases:** "swap on fluid", "fluid dex swap", "swap EURC to USDC fluid", "fluid amm swap", "Fluid兑换", "在Fluid上兑换"

**IMPORTANT:** Always run with `--dry-run` first, then ask user to confirm before proceeding.

**Usage:**
```bash
# Dry-run first
fluid --chain 8453 --dry-run swap --token-in EURC --token-out USDC --amount-in 10

# After user confirmation:
fluid --chain 8453 swap --token-in EURC --token-out USDC --amount-in 10

# wstETH -> WETH
fluid --chain 8453 swap --token-in WSTETH --token-out WETH --amount-in 0.001
```

**Key parameters:**
- `--token-in` — input token symbol (EURC, USDC, WETH, WSTETH, WEETH, FLUID, USDE)
- `--token-out` — output token symbol
- `--amount-in` — human-readable input amount
- `--slippage-bps` — slippage tolerance in basis points (default: 50 = 0.5%)

**Available pools on Base:**
| Pool | Token0 | Token1 | Pool Address |
|------|--------|--------|-------------|
| EURC/USDC | EURC | USDC | `0x2886a01a...` |
| USDe/USDC | USDE | USDC | `0x836951EB...` |
| wstETH/ETH | WSTETH | WETH | `0x667701e5...` |
| weETH/ETH | WEETH | WETH | `0x3C0441B4...` |
| FLUID/ETH | FLUID | WETH | `0xdE632C3a...` |

**What it does:**
1. For ERC-20 input: Step 1 approves pool to spend `token_in`, then calls `swapIn(swap0to1, amountIn, amountOutMin, to)`
2. For ETH input (WETH pools): sends msg.value with `swapIn`
3. After user confirmation, submits via `onchainos wallet contract-call`

**Expected output:**
```json
{
  "ok": true,
  "operation": "swap",
  "pool": "0x2886a01a...",
  "tokenIn": "EURC",
  "tokenOut": "USDC",
  "amountIn": "10",
  "approveTxHash": "0xabc...",
  "swapTxHash": "0xdef..."
}
```

---

### quote — Get DEX swap quote

**Trigger phrases:** "fluid quote", "fluid swap estimate", "how much USDC for EURC fluid", "fluid price", "Fluid报价"

**Usage:**
```bash
fluid --chain 8453 quote --token-in EURC --token-out USDC --amount-in 100
fluid --chain 8453 quote --token-in WSTETH --token-out WETH --amount-in 1
```

**What it does:**
- Simulates `swapIn` via eth_call on the Fluid DEX pool
- Returns estimated output amount
- Read-only — no confirmation needed

**Expected output:**
```json
{
  "ok": true,
  "operation": "quote",
  "tokenIn": "EURC",
  "tokenOut": "USDC",
  "amountIn": "100",
  "amountOut": "107.23",
  "note": "Quote is an estimate; actual amount may differ due to price impact and fees."
}
```

---

### borrow — Borrow from Fluid Vault (dry-run only)

**IMPORTANT:** Borrow is **dry-run only** due to liquidation risk. Always use `--dry-run`.

**Usage:**
```bash
fluid --chain 8453 --dry-run borrow --vault <vault_address> --amount 100
```

**Notes:**
- Fluid Vault borrowing requires supplying collateral to the vault first
- Liquidation can occur if collateral ratio drops below threshold
- Live execution disabled to protect users from accidental liquidation

---

### repay — Repay Fluid Vault debt (dry-run only)

**IMPORTANT:** Repay is **dry-run only**. Always use `--dry-run`.

**Usage:**
```bash
fluid --chain 8453 --dry-run repay --vault <vault_address> --amount 100
fluid --chain 8453 --dry-run repay --vault <vault_address> --all
```

---

## fToken Address Reference

### Base (chain 8453)

| fToken | Underlying | fToken Address |
|--------|------------|----------------|
| fUSDC | USDC | `0xf42f5795D9ac7e9D757dB633D693cD548Cfd9169` |
| fWETH | WETH | `0x9272D6153133175175Bc276512B2336BE3931CE9` |
| fGHO | GHO | `0x8DdbfFA3CFda2355a23d6B11105AC624BDbE3631` |
| fEURC | EURC | `0x1943FA26360f038230442525Cf1B9125b5DCB401` |

### Ethereum Mainnet (chain 1)

| fToken | Underlying | fToken Address |
|--------|------------|----------------|
| fUSDC | USDC | `0x9Fb7b4477576Fe5B32be4C1843aFB1e55F251B33` |
| fWETH | WETH | `0x90551c1795392094FE6D29B758EcCD233cFAa260` |
| fUSDT | USDT | `0x5C20B550819128074FD538Edf79791733ccEdd18` |

### Arbitrum (chain 42161)

| fToken | Underlying | fToken Address |
|--------|------------|----------------|
| fUSDC | USDC | `0x1A996cb54bb95462040408C06122D45D6Cdb6096` |
| fWETH | WETH | `0x45Df0656F8aDf017590009d2f1898eeca4F0a205` |
| fUSDT | USDT | `0x4A03F37e7d3fC243e3f99341d36f4b829BEe5E03` |

---

## Token Address Reference

### Base (8453)

| Symbol | Address |
|--------|---------|
| USDC | `0x833589fcd6edb6e08f4c7c32d4f71b54bda02913` |
| WETH | `0x4200000000000000000000000000000000000006` |
| EURC | `0x60a3E35Cc302bFA44Cb288Bc5a4F316Fdb1aDb42` |
| wstETH | `0xc1cba3fcea344f92d9239c08c0568f6f2f0ee452` |
| weETH | `0x04C0599Ae5A44757c0af6F9eC3b93da8976c150A` |

---

## Function Selectors

| Operation | Function | Selector |
|-----------|----------|----------|
| Supply (deposit) | `deposit(uint256,address)` | `0x6e553f65` |
| Withdraw partial | `withdraw(uint256,address,address)` | `0xb460af94` |
| Withdraw all (redeem) | `redeem(uint256,address,address)` | `0xba087652` |
| ERC-20 approve | `approve(address,uint256)` | `0x095ea7b3` |
| DEX swap in | `swapIn(bool,uint256,uint256,address)` | `0x2668dfaa` |
| DEX swap out | `swapOut(bool,uint256,uint256,address)` | `0x286f0e61` |

---

## Safety Rules

1. **Dry-run first**: Always simulate with `--dry-run` before any on-chain write
2. **Ask user to confirm**: Show what will happen and wait for explicit confirmation before executing
3. **Borrow/repay dry-run only**: Vault operations carry liquidation risk — never execute live
4. **Approval before deposit**: ERC-20 tokens require prior approval; plugin handles this automatically in two steps
5. **3-second delay**: Plugin waits 3 seconds after approve before deposit to avoid nonce conflicts

---

## Troubleshooting

| Error | Solution |
|-------|----------|
| `Could not resolve wallet address` | Run `onchainos wallet login` |
| `Unsupported chain ID` | Use 1 (Ethereum), 8453 (Base), or 42161 (Arbitrum) |
| `Unknown fToken` | Use symbols fUSDC, fWETH, fGHO, fEURC or provide the fToken contract address |
| `No fToken shares found` | No balance in that fToken for this address |
| `No Fluid DEX pool found` | Only supported pairs: EURC/USDC, USDe/USDC, wstETH/ETH, weETH/ETH, FLUID/ETH |
| `Borrow is only supported in --dry-run mode` | Add `--dry-run` flag; live borrow is disabled for safety |
| `eth_call error` | RPC may be rate-limited; retry or check network |
