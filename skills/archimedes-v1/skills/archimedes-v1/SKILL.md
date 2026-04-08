---
name: archimedes-v1
description: "Archimedes Finance leveraged yield protocol on Ethereum. Trigger phrases: open archimedes position, open leveraged position, archimedes yield, deposit to archimedes, close archimedes position, my archimedes positions, archimedes protocol info, archimedes available leverage, archimedes NFT positions, leveraged OUSD yield"
version: "0.1.0"
author: "skylavis-sky"
tags:
  - leverage
  - yield
  - lending
  - defi
  - ethereum
  - ousd
  - nft-position
  - curve
---

# Archimedes Finance Skill

## Overview

Archimedes Finance is an Ethereum mainnet leveraged-yield protocol. Users deposit USDC, USDT, or DAI and receive up to 10x leveraged exposure to OUSD (Origin Dollar) yield. Each position is represented as an ERC-721 NFT (PositionToken). The protocol charges an ARCH token origination fee, which can be paid automatically from the deposit or from a wallet ARCH balance.

**Chain:** Ethereum Mainnet (chain ID 1)

**Key contracts:**
| Contract | Address |
|----------|---------|
| LeverageEngine | 0x03dc7Fa99B986B7E6bFA195f39085425d8172E29 |
| Zapper | 0x624f570C24d61Ba5BF8FBFF17AA39BFc0a7b05d8 |
| PositionToken (ERC-721) | 0x14c6A3C8DBa317B87ab71E90E264D0eA7877139D |
| CDPosition | 0x229a9733063eAD8A1f769fd920eb60133fCCa3Ef |
| Coordinator | 0x58c968fADa478adb995b59Ba9e46e3Db4d6B579d |
| ParameterStore | 0xcc6Ea29928A1F6bc4796464F41b29b6d2E0ee42C |
| ARCH token | 0x73C69d24ad28e2d43D03CBf35F79fE26EBDE1011 |

---

## Pre-flight Checks

Before executing any command:

1. **Binary installed**: `archimedes --version`
2. **Wallet connected**: `onchainos wallet status`
3. **Protocol active**: `archimedes protocol-info` -- confirm availableLvUSD > 0

If wallet is not connected:
```
Please connect your wallet first: run `onchainos wallet login`
```

---

## Command Routing Table

| User Intent | Command |
|-------------|---------|
| Open a leveraged position | `archimedes open-position --amount <N> --token <USDC|USDT|DAI> --cycles <1-10>` |
| Open with ARCH fee from wallet | `archimedes open-position --amount <N> --token USDC --cycles 5 --use-arch` |
| Close / unwind a position | `archimedes close-position --token-id <NFT_ID>` |
| View my positions | `archimedes get-positions` |
| View protocol stats | `archimedes protocol-info` |

**Global flags:**
- `--from <ADDRESS>` -- wallet address (defaults to active onchainos wallet)
- `--dry-run` -- simulate without broadcasting; returns expected commands

---

## Commands

### open-position -- Open a leveraged yield position

**Trigger phrases:** "open archimedes position", "open leveraged OUSD position", "deposit to archimedes", "archimedes 5x yield", "create archimedes position"

**Usage:**
```bash
# Dry-run first (always recommended)
archimedes --dry-run open-position --amount 1000 --token USDC --cycles 5
# Execute after user confirms
archimedes open-position --amount 1000 --token USDC --cycles 5
# Pay ARCH fee from wallet instead of stablecoin
archimedes open-position --amount 1000 --token USDC --cycles 5 --use-arch
```

**Parameters:**
- `--amount` -- deposit amount in human-readable units (e.g. 1000 for 1000 USDC)
- `--token` -- stablecoin: USDC (default), USDT, or DAI
- `--cycles` -- leverage multiplier 1-10 (default: 5); higher = more yield AND more risk
- `--use-arch` -- pay origination fee in ARCH from wallet (default: fee auto-deducted from stablecoin)
- `--max-slippage-bps` -- slippage tolerance in basis points (default: 50 = 0.5%)

**What it does:**
1. Checks `getAvailableLeverage()` -- fails fast if protocol has no liquidity
2. Calls `previewZapInAmount()` to estimate ARCH fee and OUSD output
3. **Ask user to confirm** the amount, token, cycles, and estimated ARCH fee before proceeding
4. Approves stablecoin to Zapper via `onchainos wallet contract-call` (step requires user confirmation)
5. (If `--use-arch`) Approves ARCH to Zapper via `onchainos wallet contract-call` (requires user confirmation)
6. Waits 3 seconds (nonce collision protection)
7. Calls `Zapper.zapIn()` via `onchainos wallet contract-call` -- mints a PositionToken NFT (requires user confirmation)

**Important:**
- USDC/USDT use 6 decimal places; DAI uses 18
- The minted NFT ID is in the transaction receipt Transfer event; not returned directly
- minArchAmount and minOUSDAmount are set to 95% of the preview amounts

**Expected output:**
```json
{
  "ok": true,
  "dryRun": false,
  "token": "USDC",
  "amount": 1000,
  "cycles": 5,
  "previewOUSDOut": "4985.123456",
  "approveTxHash": "0xabc...",
  "zapInTxHash": "0xdef...",
  "note": "Check transaction receipt for minted PositionToken NFT ID"
}
```

---

### close-position -- Close a leveraged position and redeem OUSD

**Trigger phrases:** "close archimedes position", "unwind archimedes", "exit archimedes position #42", "redeem archimedes OUSD"

**IMPORTANT:** Always dry-run first and confirm with user before executing.

**Usage:**
```bash
# Dry-run first
archimedes --dry-run close-position --token-id 42
# Execute after confirmation
archimedes close-position --token-id 42
# With custom min OUSD return
archimedes close-position --token-id 42 --min-return 950.0
```

**Parameters:**
- `--token-id` -- PositionToken NFT ID to close
- `--min-return` -- minimum OUSD to accept (default: 95% of current position value)

**What it does:**
1. Verifies wallet owns the NFT via `ownerOf()`
2. Fetches current position value via `getOUSDTotalIncludeInterest()`
3. **Ask user to confirm** position details and minimum OUSD return before proceeding
4. Checks if LeverageEngine already has `setApprovalForAll` approval
5. If not approved: calls `PositionToken.setApprovalForAll(LeverageEngine, true)` via `onchainos wallet contract-call` (requires user confirmation)
6. Waits 3 seconds
7. Calls `LeverageEngine.unwindLeveragedPosition(tokenId, minReturnedOUSD)` via `onchainos wallet contract-call` (requires user confirmation)

**Expected output:**
```json
{
  "ok": true,
  "tokenId": "42",
  "ousdTotalWithInterest": "1050.123456",
  "lvUSDBorrowed": "4800.000000",
  "minReturnedOUSD": "997.617283",
  "setApprovalTxHash": "0xabc...",
  "unwindTxHash": "0xdef..."
}
```

---

### get-positions -- List all positions for a wallet

**Trigger phrases:** "my archimedes positions", "archimedes NFT positions", "list archimedes holdings", "archimedes portfolio"

**Usage:**
```bash
archimedes get-positions
archimedes get-positions --wallet 0xSomeAddress
```

**What it does:**
1. Calls `PositionToken.getTokenIDsArray(wallet)` to get all NFT IDs
2. For each NFT, fetches CDPosition data: OUSD principal, interest, lvUSD debt, expiry

**Expected output:**
```json
{
  "ok": true,
  "wallet": "0xabc...",
  "positionCount": 2,
  "positions": [
    {
      "tokenId": "42",
      "ousdPrinciple": "1000.000000",
      "ousdInterestEarned": "50.123456",
      "ousdTotalWithInterest": "1050.123456",
      "lvUSDBorrowed": "4800.000000",
      "expireTimestamp": "1720000000"
    }
  ]
}
```

---

### protocol-info -- Show current protocol parameters

**Trigger phrases:** "archimedes protocol info", "archimedes available leverage", "archimedes max cycles", "how much leverage archimedes has", "archimedes stats"

**Usage:**
```bash
archimedes protocol-info
```

**Expected output:**
```json
{
  "ok": true,
  "chain": "Ethereum Mainnet",
  "availableLvUSD": "125000.000000",
  "archToLevRatio": "1000000000000000000",
  "maxCycles": 10,
  "minPositionCollateralOUSD": "100.000000",
  "originationFeeRate": "1000000000000000"
}
```

---

## Do NOT use for

Do NOT use for: protocols other than Archimedes Finance, leveraged positions below 750 OUSD minimum, non-OUSD collateral types

## Safety Rules

1. **Dry-run first**: Always simulate with `--dry-run` before any on-chain write
2. **Confirm before broadcast**: Show the user what will happen and wait for explicit confirmation
3. **Check liquidity**: Run `archimedes protocol-info` first -- if availableLvUSD is low, reduce cycles
4. **Verify ownership**: `close-position` checks NFT ownership and reverts if wallet does not own it
5. **Slippage protection**: Default 0.5% (50 bps) slippage; archMinAmount and ousdMinAmount auto-set to 95% of preview
6. **3-second wait**: Built-in delay between approve and zapIn/unwind to prevent nonce collision

---

## Stablecoin Decimals Reference

| Token | Address | Decimals |
|-------|---------|---------|
| USDC | 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 | 6 |
| USDT | 0xdAC17F958D2ee523a2206206994597C13D831ec7 | 6 |
| DAI | 0x6B175474E89094C44Da98b954EedeAC495271d0F | 18 |

---

## Troubleshooting

| Error | Solution |
|-------|----------|
| `Could not resolve wallet address` | Run `onchainos wallet login` |
| `Protocol has no available lvUSD leverage` | Check `archimedes protocol-info`; try fewer cycles or wait for liquidity |
| `Wallet does not own PositionToken` | Verify NFT ID with `archimedes get-positions` |
| `Unsupported stablecoin` | Use USDC, USDT, or DAI |
| `eth_call RPC error` | RPC may be rate-limited; retry in a moment |
