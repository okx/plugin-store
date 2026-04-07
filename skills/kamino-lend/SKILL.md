---
name: kamino-lend
version: 0.1.0
description: Supply, borrow, and manage positions on Kamino Lend — the leading Solana lending protocol
---

# Kamino Lend Skill

## Overview

Kamino Lend is the leading borrowing and lending protocol on Solana. This skill enables you to:
- View lending markets and current interest rates
- Check your lending positions and health factor
- Supply assets to earn yield
- Withdraw supplied assets
- Borrow assets (dry-run preview)
- Repay borrowed assets (dry-run preview)

All on-chain operations are executed via `onchainos wallet contract-call` after explicit user confirmation.

## Pre-flight Checks

Before executing any command:
1. Ensure `kamino-lend` binary is installed and in PATH
2. Ensure `onchainos` is installed and you are logged in: `onchainos wallet balance --chain 501`
3. Wallet is on Solana mainnet (chain 501)

## Commands

### markets — View Lending Markets

Trigger phrases:
- "Show me Kamino lending markets"
- "What are the interest rates on Kamino?"
- "Kamino supply APY"
- "Kamino lending rates"

```bash
kamino-lend markets
kamino-lend markets --name "main"
```

Expected output: List of markets with supply APY, borrow APY, and TVL for each reserve.

---

### positions — View Your Positions

Trigger phrases:
- "What are my Kamino positions?"
- "Show my Kamino lending obligations"
- "My Kamino health factor"
- "How much have I borrowed on Kamino?"

```bash
kamino-lend positions
kamino-lend positions --wallet <WALLET_ADDRESS>
```

Expected output: List of obligations with deposits, borrows, and health factor.

---

### supply — Supply Assets

Trigger phrases:
- "Supply [amount] [token] to Kamino"
- "Deposit [amount] [token] on Kamino Lend"
- "Earn yield on Kamino with [token]"
- "Lend [amount] [token] on Kamino"

Before executing, **ask user to confirm** the transaction details (token, amount, current APY).

```bash
kamino-lend supply --token USDC --amount 0.01
kamino-lend supply --token SOL --amount 0.001
kamino-lend supply --token USDC --amount 0.01 --dry-run
```

Parameters:
- `--token`: Token symbol (USDC, SOL) or reserve address
- `--amount`: Amount in UI units (0.01 USDC = 0.01, NOT 10000)
- `--dry-run`: Preview without submitting (optional)
- `--wallet`: Override wallet address (optional)
- `--market`: Override market address (optional)

**Important:** After user confirmation, executes via `onchainos wallet contract-call --chain 501 --unsigned-tx <base58_tx> --force`. The transaction is fetched from Kamino API and immediately submitted (Solana blockhash expires in ~60 seconds).

---

### withdraw — Withdraw Assets

Trigger phrases:
- "Withdraw [amount] [token] from Kamino"
- "Remove my [token] from Kamino Lend"
- "Get back my [token] from Kamino"

Before executing, **ask user to confirm** the withdrawal amount and token.

```bash
kamino-lend withdraw --token USDC --amount 0.01
kamino-lend withdraw --token SOL --amount 0.001
kamino-lend withdraw --token USDC --amount 0.01 --dry-run
```

Parameters: Same as `supply`.

**Note:** Withdrawing when you have outstanding borrows may fail if it would bring health factor below 1.0. Check positions first.

After user confirmation, submits transaction via `onchainos wallet contract-call`.

---

### borrow — Borrow Assets (Dry-run)

Trigger phrases:
- "Borrow [amount] [token] from Kamino"
- "Take a loan of [amount] [token] on Kamino"
- "How much can I borrow on Kamino?"

```bash
kamino-lend borrow --token SOL --amount 0.001 --dry-run
kamino-lend borrow --token USDC --amount 0.01 --dry-run
```

**Note:** Borrowing requires prior collateral supply. Use `--dry-run` to preview. To borrow for real, omit `--dry-run` and **confirm** the transaction.

Before executing a real borrow, **ask user to confirm** and warn about liquidation risk.

---

### repay — Repay Borrowed Assets (Dry-run)

Trigger phrases:
- "Repay [amount] [token] on Kamino"
- "Pay back my [token] loan on Kamino"
- "Reduce my Kamino debt"

```bash
kamino-lend repay --token SOL --amount 0.001 --dry-run
kamino-lend repay --token USDC --amount 0.01 --dry-run
```

Before executing a real repay, **ask user to confirm** the repayment details.

---

## Error Handling

| Error | Meaning | Action |
|-------|---------|--------|
| `Kamino API deposit error: Vanilla type Kamino Lend obligation does not exist` | No prior deposits | Supply first to create obligation |
| `base64→base58 conversion failed` | API returned invalid tx | Retry; the API transaction may have expired |
| `Cannot resolve wallet address` | Not logged in to onchainos | Run `onchainos wallet balance --chain 501` to verify login |
| `Unknown token 'X'` | Unsupported token symbol | Use USDC or SOL, or pass reserve address directly |

## Routing Rules

- Use this skill for Kamino **lending** (supply/borrow/repay/withdraw)
- For Kamino **earn vaults** (automated yield strategies): use kamino-liquidity skill if available
- For general Solana token swaps: use swap/DEX skills
- Amounts are always in UI units (human-readable): 1 USDC = 1.0, not 1000000
