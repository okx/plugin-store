---
name: archimedes
description: "Deposit and withdraw from Archimedes Finance V2 protected yield vaults (ERC4626) on Ethereum mainnet. Supports WETH and crvFRAX strategies via Convex and Aura."
version: "0.1.0"
author: "ganlinux"
tags:
  - yield
  - vault
  - erc4626
  - ethereum
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Install onchainos CLI + Skills (auto-injected)

```bash
# 1. Install onchainos CLI
onchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh

# 2. Install onchainos skills (enables AI agent to use onchainos commands)
npx skills add okx/onchainos-skills --yes --global

# 3. Install plugin-store skills (enables plugin discovery and management)
npx skills add okx/plugin-store --skill plugin-store --yes --global
```

### Install archimedes binary (auto-injected)

```bash
if ! command -v archimedes >/dev/null 2>&1; then
  OS=$(uname -s | tr A-Z a-z)
  ARCH=$(uname -m)
  case "${OS}_${ARCH}" in
    darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
    darwin_x86_64) TARGET="x86_64-apple-darwin" ;;
    linux_x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
    linux_aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
  esac
  curl -fsSL "https://github.com/okx/plugin-store/releases/download/plugins/archimedes@0.1.0/archimedes-${TARGET}" -o ~/.local/bin/archimedes
  chmod +x ~/.local/bin/archimedes
fi
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/archimedes"
if [ ! -f "$REPORT_FLAG" ]; then
  mkdir -p "$HOME/.plugin-store/reported"
  # Device fingerprint → SHA256 → 32-char device ID
  DEV_RAW="$(hostname):$(uname -s):$(uname -m):$HOME"
  DEV_ID=$(echo -n "$DEV_RAW" | shasum -a 256 | head -c 32)
  # HMAC signature (obfuscated key, same as CLI binary)
  _K=$(echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | base64 -d 2>/dev/null || echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | openssl base64 -d)
  HMAC_SIG=$(echo -n "${_K}${DEV_ID}" | shasum -a 256 | head -c 8)
  DIV_ID="${DEV_ID}${HMAC_SIG}"
  unset _K
  # Report to Vercel stats
  curl -s -X POST "https://plugin-store-dun.vercel.app/install" \
    -H "Content-Type: application/json" \
    -d '{"name":"archimedes","version":"0.1.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"archimedes","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


# archimedes

## Overview

Archimedes Finance V2 is a protected yield protocol on Ethereum mainnet. It wraps Convex and Aura LP strategies inside ERC4626 vaults with an offchain auto-protection monitor. Users deposit WETH or crvFRAX, receive vault shares that appreciate over time, and can withdraw or redeem at any point.

- Read ops: direct `eth_call` against Ethereum mainnet RPC
- Write ops: after user confirmation, submits via onchainos wallet contract-call
- Non-standard ERC4626: `withdraw` and `redeem` take a 4th `minimumReceive` slippage param

## Pre-flight Checks

Before using this skill, ensure:

1. **Check onchainos**: `which onchainos` - if not found, install from https://web3.okx.com/onchainos
2. **Check binary**: `which archimedes` - if not found, install via `plugin-store install archimedes`
3. **Check wallet login**: `onchainos wallet status` - must show `loggedIn: true`; if not, run `onchainos wallet login`
4. **For write operations**: verify Ethereum mainnet (chain 1) wallet has sufficient ETH for gas

## Commands

### vaults

List all known Archimedes V2 vault addresses with underlying asset and current TVL.

```bash
archimedes vaults
archimedes vaults --rpc https://ethereum.publicnode.com
```

**When to use**: When user asks about available Archimedes vaults, yield strategies, or vault addresses.

**Parameters:**
- `--rpc`: Optional custom Ethereum RPC URL

**Output**: JSON list of vaults with name, vault address, underlying symbol, underlying address, and TVL.

**Example**:
```bash
archimedes vaults
# Returns: [{"name":"WETH ETH+ Strategy (Convex)","address":"0xfA364CB...","tvl":"1.234"}]
```

---

### positions

Show wallet's share balance and underlying asset value across all Archimedes vaults.

```bash
archimedes positions
archimedes positions --wallet 0xAbCd...1234
archimedes positions --rpc https://ethereum.publicnode.com
```

**When to use**: When user asks about their Archimedes position, vault balance, or deposited assets.

**Parameters:**
- `--wallet`: Optional wallet address to query (defaults to logged-in wallet)
- `--rpc`: Optional custom Ethereum RPC URL

**Output**: JSON list of positions with vault name, shares held, underlying value, and vault TVL.

**Example**:
```bash
archimedes positions
# Returns: [{"vault":"WETH ETH+ Strategy","shares":"0.001","underlying_value":"0.001 WETH"}]
```

---

### deposit

Deposit underlying assets into an Archimedes V2 vault. Executes ERC-20 approve followed by vault deposit.

```bash
archimedes deposit --vault <ADDR> --amount <AMOUNT> [--from <ADDR>] [--rpc <URL>] [--dry-run]
```

**When to use**: When user wants to deposit WETH or crvFRAX into an Archimedes vault to earn yield.

**Parameters:**
- `--vault`: Vault contract address (use `archimedes vaults` to list)
- `--amount`: Amount of underlying asset in human-readable form (e.g. "0.01")
- `--from`: Wallet address (receiver); defaults to logged-in wallet
- `--rpc`: Optional custom Ethereum RPC URL
- `--dry-run`: Preview calldata without submitting

**Flow:**
1. Run with `--dry-run` to preview both transactions
2. **Ask user to confirm** Step 1 (ERC-20 approve) before broadcasting
3. Execute: token.approve(vault, amount)
4. Wait 15 seconds for confirmation on Ethereum mainnet
5. **Ask user to confirm** Step 2 (vault deposit) before broadcasting
6. Execute: vault.deposit(assets, receiver)

**Example**:
```bash
archimedes deposit --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --amount 0.001 --dry-run
archimedes deposit --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --amount 0.001
```

**Output**: JSON with approve tx hash, deposit tx hash, expected shares received.

---

### withdraw

Withdraw underlying assets from a vault by specifying asset amount. Uses non-standard 4-param `withdraw(assets, receiver, owner, minimumReceive)`.

```bash
archimedes withdraw --vault <ADDR> --amount <AMOUNT> [--from <ADDR>] [--slippage-bps <N>] [--rpc <URL>] [--dry-run]
```

**When to use**: When user wants to withdraw a specific amount of underlying assets from an Archimedes vault.

**Parameters:**
- `--vault`: Vault contract address
- `--amount`: Amount of underlying asset to withdraw (human-readable)
- `--from`: Wallet address (receiver and owner); defaults to logged-in wallet
- `--slippage-bps`: Slippage tolerance in basis points (default: 50 = 0.5%); use 0 to skip minimum
- `--rpc`: Optional custom Ethereum RPC URL
- `--dry-run`: Simulate without broadcasting

**Flow:**
1. Run with `--dry-run` to preview calldata
2. **Ask user to confirm** the withdrawal before broadcasting
3. Execute: vault.withdraw(assets, receiver, owner, minimumReceive)

**Example**:
```bash
archimedes withdraw --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --amount 0.001 --dry-run
archimedes withdraw --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --amount 0.001
archimedes withdraw --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --amount 0.001 --slippage-bps 100
```

**Output**: JSON with tx hash, assets requested, minimum receive threshold.

---

### redeem

Redeem vault shares for underlying assets. Redeems all shares by default. Uses non-standard 4-param `redeem(shares, receiver, owner, minimumReceive)`.

```bash
archimedes redeem --vault <ADDR> [--shares <AMOUNT>] [--from <ADDR>] [--slippage-bps <N>] [--rpc <URL>] [--dry-run]
```

**When to use**: When user wants to exit an Archimedes vault by redeeming shares.

**Parameters:**
- `--vault`: Vault contract address
- `--shares`: Number of shares to redeem (omit to redeem all)
- `--from`: Wallet address (receiver and owner); defaults to logged-in wallet
- `--slippage-bps`: Slippage tolerance in basis points (default: 50 = 0.5%); use 0 to skip minimum
- `--rpc`: Optional custom Ethereum RPC URL
- `--dry-run`: Simulate without broadcasting

**Flow:**
1. Run with `--dry-run` to preview calldata
2. **Ask user to confirm** the redemption before broadcasting
3. Execute: vault.redeem(shares, receiver, owner, minimumReceive)

**Example**:
```bash
archimedes redeem --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --dry-run
archimedes redeem --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269
archimedes redeem --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --shares 0.5
```

**Output**: JSON with tx hash, shares redeemed, expected underlying assets received.

---

## Examples

### Example 1: Deposit WETH into Archimedes Vault

```bash
# Step 1: List available vaults
archimedes vaults

# Step 2: Check current position (before)
archimedes positions

# Step 3: Preview deposit
archimedes deposit --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --amount 0.01 --dry-run

# Step 4: Execute deposit (will prompt for confirmation twice)
archimedes deposit --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --amount 0.01
```

### Example 2: Redeem All Shares

```bash
# Step 1: Check current positions to find vault address and shares held
archimedes positions

# Step 2: Preview redeem
archimedes redeem --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269 --dry-run

# Step 3: Execute redeem (will prompt for confirmation)
archimedes redeem --vault 0xfA364CBca915f17fEc356E35B61541fC6D4D8269
```

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| "Unknown vault address" | Vault address not in hardcoded list | Run `archimedes vaults` to get valid addresses |
| "Insufficient WETH balance" | Not enough underlying token | Check balance before depositing |
| "deposit failed" | Approve tx not confirmed yet | Retry; 15s wait between approve and deposit |
| "withdraw failed: revert" | minimumReceive too high | Increase `--slippage-bps` or use `--slippage-bps 0` |
| "Could not resolve wallet" | Not logged into onchainos | Run `onchainos wallet login` |
| RPC timeout | Public RPC unavailable | Use `--rpc` with alternative endpoint |

## Security Notices

- This plugin interacts with Archimedes Finance V2 vaults on Ethereum mainnet. Deposits involve real funds.
- All write operations require explicit user confirmation before broadcasting.
- `--dry-run` mode is available for all write commands to preview calldata safely.
- Vault factory is inactive; vault addresses are hardcoded (3 known vaults). Verify addresses on-chain before transacting.
- Funds sit idle in vault until offchain monitor triggers rebalance; yield accrues after rebalance only.
- `minimumReceive` slippage protection is applied on all withdrawals (default 0.5%); use `--slippage-bps 0` to disable.

## Known Vaults (Ethereum Mainnet)

| Vault | Address | Underlying |
|-------|---------|------------|
| WETH ETH+ Strategy (Convex) | `0xfA364CBca915f17fEc356E35B61541fC6D4D8269` | WETH |
| WETH Aura Weighted Strategy | `0x83FeD5139eD14162198Bd0a54637c22cA854E2f6` | WETH |
| alUSD FRAXBP Strategy (Convex) | `0x2E04e0aEa173F95A23043576138539fBa60D930a` | crvFRAX |
