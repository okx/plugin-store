---
name: morpho-base
description: "Supply, borrow and earn yield on Morpho V1 on Base - permissionless isolated lending on the Base network. Trigger phrases: supply to morpho base, deposit to morpho vault on base, borrow from morpho base, repay morpho base loan, my morpho base positions, morpho base interest rates, claim morpho base rewards, morpho base markets, metamorpho vaults on base. Chinese: zai Morpho Base cunkuan, Base Morpho jiekuan, huan Morpho Base kuan, wode Morpho Base canwei"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

# Morpho Base Skill

## Overview

Morpho V1 on Base is a permissionless lending protocol operating on the Base network. It uses two layers:

- **Morpho Blue**  -  isolated lending markets identified by `MarketParams (loanToken, collateralToken, oracle, irm, lltv)`. Users supply collateral, borrow, and repay.
- **MetaMorpho**  -  ERC-4626 vaults curated by risk managers (Gauntlet, Moonwell, Steakhouse, etc.) that aggregate liquidity across Morpho Blue markets.

**Supported chain:** Base (chain ID 8453)

**Architecture:**
- Write operations (supply, withdraw, borrow, repay, supply-collateral, claim-rewards) → after user confirmation, submits via `onchainos wallet contract-call`
- ERC-20 approvals → after user confirmation, submits via `onchainos wallet contract-call` before the main operation
- Read operations (positions, markets, vaults) → direct GraphQL query to `https://blue-api.morpho.org/graphql`; no confirmation needed

---

## Pre-flight Checks

Before executing any command, verify:

1. **Binary installed**: `morpho-base --version`  -  if not found, instruct user to install the plugin
2. **Wallet connected**: `onchainos wallet status`  -  confirm logged in and active address is set

If the wallet is not connected, output:
```
Please connect your wallet first: run `onchainos wallet login`
```

---

## Command Routing Table

| User Intent | Command |
|-------------|---------|
| Supply / deposit to MetaMorpho vault on Base | `morpho-base supply --vault <addr> --asset <sym> --amount <n>` |
| Withdraw from MetaMorpho vault on Base | `morpho-base withdraw --vault <addr> --asset <sym> --amount <n>` |
| Withdraw all from vault | `morpho-base withdraw --vault <addr> --asset <sym> --all` |
| Borrow from Morpho Blue market on Base | `morpho-base borrow --market-id <hex> --amount <n>` |
| Repay Morpho Blue debt on Base | `morpho-base repay --market-id <hex> --amount <n>` |
| Repay all Morpho Blue debt | `morpho-base repay --market-id <hex> --all` |
| View positions and health factor | `morpho-base positions` |
| List Base markets with APYs | `morpho-base markets` |
| Filter markets by asset | `morpho-base markets --asset USDC` |
| Supply collateral to Blue market | `morpho-base supply-collateral --market-id <hex> --amount <n>` |
| Claim Merkl rewards | `morpho-base claim-rewards` |
| List MetaMorpho vaults on Base | `morpho-base vaults` |
| Filter vaults by asset | `morpho-base vaults --asset USDC` |

**Global flags (always available):**
- `--chain 8453`  -  Base network (default, only supported chain)
- `--from <ADDRESS>`  -  wallet address (defaults to active onchainos wallet)
- `--dry-run`  -  simulate without broadcasting

---

## Health Factor Rules

The health factor (HF) is a numeric value representing the safety of a borrowing position:
- **HF ≥ 1.1** → `safe`  -  position is healthy
- **1.05 ≤ HF < 1.1** → `warning`  -  elevated liquidation risk
- **HF < 1.05** → `danger`  -  high liquidation risk

**Rules:**
- **Always** check health factor before borrow operations
- **Warn** when post-action estimated HF < 1.1
- **Block** (require explicit user confirmation) when current HF < 1.05
- **Never** execute borrow if HF would drop below 1.0

---

## Execution Flow for Write Operations

For all write operations (supply, withdraw, borrow, repay, supply-collateral, claim-rewards):

1. Run with `--dry-run` first to preview the transaction
2. **Ask user to confirm** before executing on-chain
3. Execute only after receiving explicit user approval
4. Report transaction hash(es) and outcome

---

## Commands

### supply  -  Deposit to MetaMorpho vault on Base

**Trigger phrases:** "supply to morpho base", "deposit to morpho on base", "earn yield on morpho base", "supply usdc to morpho base vault", "在Morpho Base存款"

**Usage:**
```bash
# Always dry-run first, then ask user to confirm before proceeding
morpho-base --dry-run supply --vault 0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183 --asset USDC --amount 10
# After user confirmation:
morpho-base supply --vault 0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183 --asset USDC --amount 10
```

**Key parameters:**
- `--vault`  -  MetaMorpho vault address (see Well-Known Vaults below)
- `--asset`  -  token symbol (USDC, WETH, cbETH, cbBTC) or ERC-20 address
- `--amount`  -  human-readable amount (e.g. 10 for 10 USDC)

**What it does:**
1. Resolves token decimals from on-chain `decimals()` call
2. Step 1: Approves vault to spend the token  -  after user confirmation, submits via `onchainos wallet contract-call`
3. Step 2: Calls `deposit(assets, receiver)` (ERC-4626)  -  after user confirmation, submits via `onchainos wallet contract-call`

**Expected output:**
```json
{
  "ok": true,
  "operation": "supply",
  "vault": "0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183",
  "asset": "USDC",
  "amount": "10",
  "approveTxHash": "0xabc...",
  "supplyTxHash": "0xdef..."
}
```

---

### withdraw  -  Withdraw from MetaMorpho vault on Base

**Trigger phrases:** "withdraw from morpho base", "redeem metamorpho base", "take out from morpho base vault", "从Morpho Base提款"

**Usage:**
```bash
# Partial withdrawal  -  dry-run first, then ask user to confirm before proceeding
morpho-base --dry-run withdraw --vault 0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183 --asset USDC --amount 5
# After user confirmation:
morpho-base withdraw --vault 0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183 --asset USDC --amount 5

# Full withdrawal  -  redeem all shares
morpho-base withdraw --vault 0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183 --asset USDC --all
```

**Key parameters:**
- `--vault`  -  MetaMorpho vault address
- `--asset`  -  token symbol or ERC-20 address
- `--amount`  -  partial withdrawal amount (mutually exclusive with `--all`)
- `--all`  -  redeem entire share balance

**Notes:**
- MetaMorpho V2 vaults return `0` for `maxWithdraw()`. The plugin uses `balanceOf` + `convertToAssets` to determine share balance for `--all`.
- Partial withdrawal calls `withdraw(assets, receiver, owner)`.
- Full withdrawal calls `redeem(shares, receiver, owner)`.
- After user confirmation, submits via `onchainos wallet contract-call`.

**Expected output:**
```json
{
  "ok": true,
  "operation": "withdraw",
  "vault": "0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183",
  "asset": "USDC",
  "amount": "5",
  "txHash": "0xabc..."
}
```

---

### borrow  -  Borrow from Morpho Blue market on Base

**Trigger phrases:** "borrow from morpho base", "get a loan on morpho blue base", "从Morpho Base借款"

**IMPORTANT:** Always run with `--dry-run` first, then ask user to confirm before executing.

**Usage:**
```bash
# Dry-run first
morpho-base --dry-run borrow --market-id 0x9103c3b4e834476c9a62ea009ba2c884ee42e94e6e314a26f04d312434191836 --amount 1
# After user confirmation:
morpho-base borrow --market-id 0x9103c3b4e834476c9a62ea009ba2c884ee42e94e6e314a26f04d312434191836 --amount 1
```

**Key parameters:**
- `--market-id`  -  Market unique key (bytes32 hex from `morpho-base markets`)
- `--amount`  -  human-readable borrow amount in loan token units

**What it does:**
1. Fetches `MarketParams` for the market from the Morpho GraphQL API
2. Calls `borrow(marketParams, assets, 0, onBehalf, receiver)` on Morpho Blue
3. After user confirmation, submits via `onchainos wallet contract-call`

**Pre-condition:** User must have supplied sufficient collateral for the market.

---

### repay  -  Repay Morpho Blue debt on Base

**Trigger phrases:** "repay morpho base loan", "pay back morpho base debt", "还Morpho Base款"

**IMPORTANT:** Always run with `--dry-run` first, then ask user to confirm before proceeding.

**Usage:**
```bash
# Repay partial amount  -  dry-run first
morpho-base --dry-run repay --market-id 0x9103... --amount 0.5
# After user confirmation:
morpho-base repay --market-id 0x9103... --amount 0.5

# Repay all outstanding debt
morpho-base repay --market-id 0x9103... --all
```

**Notes:**
- Full repayment uses `repay(marketParams, 0, borrowShares, onBehalf, 0x)` (shares mode) to avoid leaving dust.
- A 0.5% approval buffer is added to cover accrued interest.
- Step 1 approves Morpho Blue to spend the loan token  -  after user confirmation, submits via `onchainos wallet contract-call`.
- Step 2 calls `repay(...)`  -  after user confirmation, submits via `onchainos wallet contract-call`.

---

### positions  -  View positions and health factors

**Trigger phrases:** "my morpho base positions", "morpho base portfolio", "morpho base health factor", "我的Morpho Base仓位", "Base链Morpho持仓"

**Usage:**
```bash
morpho-base positions
morpho-base positions --from 0xYourAddress
```

**What it does:**
- Queries the Morpho GraphQL API for Morpho Blue market positions and MetaMorpho vault positions on Base
- Returns borrow/supply amounts, and collateral for each position
- Read-only  -  no confirmation needed

---

### markets  -  List Morpho Blue markets on Base

**Trigger phrases:** "morpho base markets", "morpho base interest rates", "morpho base borrow rates", "Morpho Base利率"

**Usage:**
```bash
# List all Base markets
morpho-base markets
# Filter by loan asset
morpho-base markets --asset USDC
morpho-base markets --asset WETH
```

**What it does:**
- Queries the Morpho GraphQL API for top markets on Base ordered by TVL
- Returns supply APY, borrow APY, utilization, and LLTV for each market
- Read-only  -  no confirmation needed

---

### supply-collateral  -  Supply collateral to Morpho Blue on Base

**Trigger phrases:** "supply collateral to morpho base", "add collateral morpho blue base", "Morpho Base存入抵押品"

**IMPORTANT:** Always run with `--dry-run` first, then ask user to confirm before executing.

**Usage:**
```bash
# Dry-run first
morpho-base --dry-run supply-collateral --market-id 0x9103... --amount 0.001
# After user confirmation:
morpho-base supply-collateral --market-id 0x9103... --amount 0.001
```

**What it does:**
1. Fetches `MarketParams` from the Morpho GraphQL API
2. Step 1: Approves Morpho Blue to spend collateral token  -  after user confirmation, submits via `onchainos wallet contract-call`
3. Step 2: Calls `supplyCollateral(marketParams, assets, onBehalf, 0x)`  -  after user confirmation, submits via `onchainos wallet contract-call`

---

### claim-rewards  -  Claim Merkl rewards on Base

**Trigger phrases:** "claim morpho base rewards", "collect morpho base rewards", "领取Morpho Base奖励"

**IMPORTANT:** Always run with `--dry-run` first, then ask user to confirm before executing.

**Usage:**
```bash
# Dry-run first
morpho-base --dry-run claim-rewards
# After user confirmation:
morpho-base claim-rewards
```

**What it does:**
1. Calls `GET https://api.merkl.xyz/v4/claim?user=<addr>&chainId=8453` to fetch claimable rewards and Merkle proofs
2. Encodes `claim(users[], tokens[], claimable[], proofs[][])` calldata for the Merkl Distributor
3. After user confirmation, submits via `onchainos wallet contract-call` to the Merkl Distributor

---

### vaults  -  List MetaMorpho vaults on Base

**Trigger phrases:** "morpho base vaults", "metamorpho vaults on base", "list morpho base vaults", "Base链MetaMorpho金库"

**Usage:**
```bash
# List all vaults on Base
morpho-base vaults
# Filter by asset
morpho-base vaults --asset USDC
morpho-base vaults --asset WETH
```

**What it does:**
- Queries the Morpho GraphQL API for MetaMorpho vaults on Base ordered by TVL
- Returns APY, total assets, and curator info for each vault
- Read-only  -  no confirmation needed

---

## Well-Known Vault Addresses (Base, chain 8453)

| Vault | Asset | Address |
|-------|-------|---------|
| Moonwell Flagship USDC | USDC | `0xc1256Ae5FF1cf2719D4937adb3bbCCab2E00A2Ca` |
| Steakhouse USDC | USDC | `0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183` |
| Base WETH | WETH | `0x3aC2bBD41D7A92326dA602f072D40255Dd8D23a2` |
| Moonwell Flagship ETH | WETH | `0xa0E430870c4604CcfC7B38Ca7845B1FF653D0ff1` |

---

## Token Address Reference (Base, chain 8453)

| Symbol | Address |
|--------|---------|
| WETH | `0x4200000000000000000000000000000000000006` |
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` |
| cbETH | `0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22` |
| cbBTC | `0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf` |

---

## Safety Rules

1. **Dry-run first**: Always simulate with `--dry-run` before any on-chain write
2. **Ask user to confirm**: Show the user what will happen and wait for explicit confirmation before executing
3. **Never borrow without checking collateral**: Ensure sufficient collateral is supplied first
4. **Warn at low HF**: Explicitly warn user when health factor < 1.1 after simulated borrow
5. **Full repay with shares**: Use `--all` for full repayment to avoid dust from interest rounding
6. **Approval buffer**: Repay automatically adds 0.5% buffer to approval amount for accrued interest
7. **MarketParams from API**: Market parameters are always fetched from the Morpho GraphQL API at runtime  -  never hardcoded

---

## Troubleshooting

| Error | Solution |
|-------|----------|
| `Could not resolve active wallet` | Run `onchainos wallet login` |
| `Unsupported chain ID` | This plugin only supports Base (8453) |
| `Failed to fetch market from Morpho API` | Check market ID is a valid bytes32 hex; run `morpho-base markets` to list valid market IDs |
| `No position found for this market` | No open position in the specified market |
| `No claimable rewards found` | No unclaimed rewards for this address on Base |
| `eth_call RPC error` | RPC endpoint may be rate-limited; retry or check network |
| `Unknown asset symbol` | Provide the ERC-20 contract address instead of symbol |
