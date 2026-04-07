---
name: aave-v3
description: "Aave V3 lending and borrowing. Trigger phrases: supply to aave, deposit to aave, borrow from aave, repay aave loan, aave health factor, my aave positions, aave interest rates, enable emode, disable collateral, claim aave rewards. Chinese: 在Aave存款, Aave借款, 还款, 健康因子, 我的Aave仓位, Aave利率"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

# Aave V3 Skill

## Overview

Aave V3 is the leading decentralized lending protocol with over $43B TVL. This skill lets users supply assets to earn yield, borrow against collateral, manage health factors, and monitor positions — all via the `aave-v3` binary and `onchainos` CLI.

**Supported chains:**

| Chain | Chain ID |
|-------|----------|
| Ethereum Mainnet | 1 |
| Polygon | 137 |
| Arbitrum One | 42161 |
| Base | 8453 (default) |

**Architecture:**
- Supply / Withdraw / Borrow / Repay / Set Collateral / Set E-Mode → `aave-v3` binary constructs ABI calldata; **ask user to confirm** before submitting via `onchainos wallet contract-call` to Aave Pool
- Supply / Repay first **ask user to confirm**, then approve the ERC-20 token via `wallet contract-call` before the Pool call
- Claim Rewards → `onchainos defi collect --platform-id <id>` (platform-id from `defi positions`)
- Health Factor / Reserves / Positions → `aave-v3` binary makes read-only `eth_call` via public RPC
- Pool address is always resolved at runtime via `PoolAddressesProvider.getPool()` — never hardcoded

---

## Pre-flight Checks

Before executing any command, verify:

1. **Binary installed**: `aave-v3 --version` — if not found, instruct user to install the plugin
2. **Wallet connected**: `onchainos wallet status` — confirm logged in and active address is set
3. **Chain supported**: chain ID must be one of 1, 137, 42161, 8453

If the wallet is not connected, output:
```
Please connect your wallet first: run `onchainos wallet login`
```

---

## Command Routing Table

| User Intent | Command |
|-------------|---------|
| Supply / deposit / lend asset | `aave-v3 supply --asset <ADDRESS> --amount <AMOUNT>` |
| Withdraw / redeem aTokens | `aave-v3 withdraw --asset <SYMBOL> --amount <AMOUNT>` |
| Borrow asset | `aave-v3 borrow --asset <ADDRESS> --amount <AMOUNT>` |
| Repay debt | `aave-v3 repay --asset <ADDRESS> --amount <AMOUNT>` |
| Repay all debt | `aave-v3 repay --asset <ADDRESS> --all` |
| Check health factor | `aave-v3 health-factor` |
| View positions | `aave-v3 positions` |
| List reserve rates / APYs | `aave-v3 reserves` |
| Enable/disable collateral | `aave-v3 set-collateral --asset <ADDRESS> --enable <true/false>` |
| Set E-Mode | `aave-v3 set-emode --category <ID>` |
| Claim rewards | `aave-v3 claim-rewards` |

**Global flags (always available):**
- `--chain <CHAIN_ID>` — target chain (default: 8453 Base)
- `--from <ADDRESS>` — wallet address (defaults to active onchainos wallet)
- `--dry-run` — simulate without broadcasting

---

## Health Factor Rules

The health factor (HF) is a numeric value representing the safety of a borrowing position:
- **HF ≥ 1.1** → `safe` — position is healthy
- **1.05 ≤ HF < 1.1** → `warning` — elevated liquidation risk
- **HF < 1.05** → `danger` — high liquidation risk

**Rules:**
- **Always** check health factor before borrow or set-collateral operations
- **Warn** when post-action estimated HF < 1.1
- **Block** (require explicit user confirmation) when current HF < 1.05
- **Never** execute borrow if HF would drop below 1.0

To check health factor:
```bash
aave-v3 --chain 1 health-factor --from 0xYourAddress
```

---

## Commands

### supply — Deposit to earn interest

**Trigger phrases:** "supply to aave", "deposit to aave", "lend on aave", "earn yield on aave", "在Aave存款", "在Aave存入"

**Usage:**
```bash
aave-v3 --chain 8453 supply --asset USDC --amount 1000
aave-v3 --chain 8453 --dry-run supply --asset USDC --amount 1000
```

**Key parameters:**
- `--asset` — token symbol (e.g. USDC, WETH) or ERC-20 address
- `--amount` — human-readable amount (e.g. 1000 for 1000 USDC)

**What it does:**
1. Resolves token contract address via `onchainos token search` (or uses address directly if provided)
2. Resolves Pool address at runtime via `PoolAddressesProvider.getPool()`
3. **Ask user to confirm** the supply amount and asset before proceeding with on-chain transactions.
4. Approves token to Pool: `onchainos wallet contract-call` → ERC-20 `approve(pool, amount)`
5. Deposits to Pool: `onchainos wallet contract-call` → `Pool.supply(asset, amount, onBehalfOf, 0)`

**Expected output:**
```json
{
  "ok": true,
  "approveTxHash": "0xabc...",
  "supplyTxHash": "0xdef...",
  "asset": "USDC",
  "tokenAddress": "0x833589...",
  "amount": 1000,
  "poolAddress": "0xa238dd..."
}
```

---

### withdraw — Redeem aTokens

**Trigger phrases:** "withdraw from aave", "redeem aave", "take out from aave", "从Aave提款"

**Usage:**
```bash
aave-v3 --chain 8453 withdraw --asset USDC --amount 500
aave-v3 --chain 8453 withdraw --asset USDC --all
```

**Key parameters:**
- `--asset` — token symbol or ERC-20 address
- `--amount` — partial withdrawal amount
- `--all` — withdraw the full balance

**Expected output:**
```json
{
  "ok": true,
  "txHash": "0xabc...",
  "asset": "USDC",
  "amount": "500"
}
```

---

### borrow — Borrow against collateral

**Trigger phrases:** "borrow from aave", "get a loan on aave", "从Aave借款", "Aave借贷"

**IMPORTANT:** Always run with `--dry-run` first, then confirm with user before executing.

**Usage:**
```bash
# Always dry-run first
aave-v3 --chain 42161 --dry-run borrow --asset 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 --amount 0.5
# Then execute after user confirms
aave-v3 --chain 42161 borrow --asset 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 --amount 0.5
```

**Key parameters:**
- `--asset` — ERC-20 contract address (checksummed). Borrow and repay require the address, not symbol.
- `--amount` — human-readable amount in token units (0.5 WETH = `0.5`)

**Notes:**
- Interest rate mode is always 2 (variable) — stable rate is deprecated in Aave V3.1+
- Pool address is resolved at runtime from PoolAddressesProvider; never hardcoded

**Expected output:**
```json
{
  "ok": true,
  "txHash": "0xabc...",
  "asset": "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
  "borrowAmount": 0.5,
  "currentHealthFactor": "1.8500",
  "healthFactorStatus": "safe",
  "availableBorrowsUSD": "1240.50"
}
```

---

### repay — Repay borrowed debt

**Trigger phrases:** "repay aave loan", "pay back aave debt", "还Aave款", "偿还Aave"

**IMPORTANT:** Always run with `--dry-run` first.

**Usage:**
```bash
# Repay specific amount
aave-v3 --chain 137 --dry-run repay --asset 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 --amount 1000
# Repay all debt
aave-v3 --chain 137 repay --asset 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 --all
```

**Key parameters:**
- `--asset` — ERC-20 contract address of the debt token
- `--amount` — partial repay amount
- `--all` — repay full outstanding balance (uses uint256.max)

**Notes:**
- ERC-20 approval is checked automatically; if insufficient, an approve tx is submitted first
- `--all` repay uses the wallet's actual token balance (not uint256.max) to avoid revert when accrued interest exceeds the wallet balance
- Always pass the ERC-20 address for `--asset`, not the symbol

**Expected output:**
```json
{
  "ok": true,
  "txHash": "0xabc...",
  "asset": "0x2791...",
  "repayAmount": "all (1005230000)",
  "totalDebtBefore": "1005.23",
  "approvalExecuted": true
}
```

---

### health-factor — Check account health

**Trigger phrases:** "aave health factor", "am i at risk of liquidation", "check aave position", "健康因子", "清算风险"

**Usage:**
```bash
aave-v3 --chain 1 health-factor
aave-v3 --chain 1 health-factor --from 0xSomeAddress
```

**Expected output:**
```json
{
  "ok": true,
  "chain": "Ethereum Mainnet",
  "healthFactor": "1.85",
  "healthFactorStatus": "safe",
  "totalCollateralUSD": "10000.00",
  "totalDebtUSD": "5400.00",
  "availableBorrowsUSD": "2000.00",
  "currentLiquidationThreshold": "82.50%",
  "loanToValue": "75.00%"
}
```

---

### reserves — List market rates and APYs

**Trigger phrases:** "aave interest rates", "aave supply rates", "aave borrow rates", "Aave利率", "Aave市场"

**Usage:**
```bash
# All reserves
aave-v3 --chain 8453 reserves
# Filter by symbol
aave-v3 --chain 8453 reserves --asset USDC
# Filter by address
aave-v3 --chain 8453 reserves --asset 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
```

**Expected output:**
```json
{
  "ok": true,
  "chain": "Base",
  "chainId": 8453,
  "reserveCount": 12,
  "reserves": [
    {
      "underlyingAsset": "0x833589...",
      "supplyApy": "3.2500%",
      "variableBorrowApy": "5.1200%"
    }
  ]
}
```

---

### positions — View current positions

**Trigger phrases:** "my aave positions", "aave portfolio", "我的Aave仓位", "Aave持仓"

**Usage:**
```bash
aave-v3 --chain 8453 positions
aave-v3 --chain 1 positions --from 0xSomeAddress
```

**Expected output:**
```json
{
  "ok": true,
  "chain": "Base",
  "healthFactor": "1.85",
  "healthFactorStatus": "safe",
  "totalCollateralUSD": "10000.00",
  "totalDebtUSD": "5400.00",
  "positions": { ... }
}
```

---

### set-collateral — Enable or disable collateral

**Trigger phrases:** "disable collateral on aave", "use asset as collateral", "关闭Aave抵押"

**IMPORTANT:** Always check health factor first. Disabling collateral with outstanding debt may trigger liquidation.

**Usage:**
```bash
# Dry-run first
aave-v3 --chain 1 --dry-run set-collateral --asset 0x514910771AF9Ca656af840dff83E8264EcF986CA --enable false
# Execute after confirmation
aave-v3 --chain 1 set-collateral --asset 0x514910771AF9Ca656af840dff83E8264EcF986CA --enable false
# Shorthand: --disable is equivalent to --enable false
aave-v3 --chain 1 set-collateral --asset 0x514910771AF9Ca656af840dff83E8264EcF986CA --disable
# Enable collateral
aave-v3 --chain 1 set-collateral --asset 0x514910771AF9Ca656af840dff83E8264EcF986CA --enable true
```

**Notes:**
- `--enable` accepts an explicit boolean value: `--enable true` or `--enable false`
- `--disable` is a convenience alias for `--enable false`

---

### set-emode — Set efficiency mode

**Trigger phrases:** "enable emode on aave", "aave efficiency mode", "stablecoin emode", "Aave效率模式"

**E-Mode categories:**
- `0` = No E-Mode (default)
- `1` = Stablecoins (higher LTV for correlated stablecoins)
- `2` = ETH-correlated assets

**Usage:**
```bash
aave-v3 --chain 8453 --dry-run set-emode --category 1
aave-v3 --chain 8453 set-emode --category 1
```

---

### claim-rewards — Claim accrued rewards

**Trigger phrases:** "claim aave rewards", "collect aave rewards", "领取Aave奖励"

**Usage:**
```bash
aave-v3 --chain 8453 claim-rewards
aave-v3 --chain 8453 --dry-run claim-rewards
```

---

## Asset Address Reference

For borrow and repay, you need ERC-20 contract addresses. Common addresses:

### Base (8453)
| Symbol | Address |
|--------|---------|
| USDC | 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 |
| WETH | 0x4200000000000000000000000000000000000006 |
| cbBTC | 0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf |

### Arbitrum (42161)
| Symbol | Address |
|--------|---------|
| USDC | 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 |
| WETH | 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 |
| WBTC | 0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f |

### Polygon (137)
| Symbol | Address |
|--------|---------|
| USDC | 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 |
| WETH | 0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619 |
| WMATIC | 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270 |

### Ethereum (1)
| Symbol | Address |
|--------|---------|
| USDC | 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 |
| WETH | 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 |
| LINK | 0x514910771AF9Ca656af840dff83E8264EcF986CA |

---

## Safety Rules

1. **Dry-run first**: Always simulate with `--dry-run` before any on-chain write
2. **Confirm before broadcast**: Show the user what will happen and wait for explicit confirmation
3. **Never borrow if HF < 1.5 without warning**: Explicitly warn user of liquidation risk
4. **Block at HF < 1.05**: Require explicit override from user before proceeding
5. **Full repay safety**: Use `--all` flag for full repay — avoids underpayment due to accrued interest
6. **Collateral warning**: Before disabling collateral, simulate health factor impact
7. **ERC-20 approval**: repay automatically handles approval; inform user if approval tx is included
8. **Pool address is never hardcoded**: Resolved at runtime from PoolAddressesProvider

---

## Troubleshooting

| Error | Solution |
|-------|----------|
| `Could not resolve active wallet` | Run `onchainos wallet login` |
| `No Aave V3 investment product found` | Check chain ID; run `onchainos defi search --platform aave --chain <id>` |
| `Unsupported chain ID` | Use chain 1, 137, 42161, or 8453 |
| `No borrow capacity available` | Supply collateral first or repay existing debt |
| `eth_call RPC error` | RPC endpoint may be rate-limited; retry or check network |
