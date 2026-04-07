---
name: stader
description: "Stake ETH with Stader liquid staking protocol to receive ETHx on Ethereum. Trigger phrases: stake ETH Stader, ETHx staking, stake with Stader, Stader liquid staking, unstake ETHx, claim Stader withdrawal, Stader exchange rate, Stader position, ETHx balance. Chinese: 质押ETH到Stader, Stader流动质押, 查看Stader仓位, 领取Stader提款"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Overview

Stader is a liquid staking protocol on Ethereum. Users deposit ETH to receive ETHx, a liquid staking token that accrues staking rewards. ETHx can be used in DeFi while continuing to earn Ethereum staking yields.

**Supported chain:** Ethereum Mainnet (chain ID 1)

**Key contracts:**
- StaderStakePoolsManager: `0xcf5EA1b38380f6aF39068375516Daf40Ed70D299`
- UserWithdrawManager: `0x9F0491B32DBce587c50c4C43AB303b06478193A7`
- ETHx Token: `0xA35b1B31Ce002FBF2058D22F30f95D405200A15b`

---

## Architecture

- **Read ops** (`rates`, `positions`) → direct `eth_call` via public RPC; no wallet or confirmation needed
- **Write ops** (`stake`, `unstake`, `claim`) → after user confirmation, submits via `onchainos wallet contract-call`
- All amounts in wei (18 decimal places)
- Minimum deposit: 0.0001 ETH (protocol enforced)

---

## Commands

### rates — View exchange rate and protocol stats

**Trigger phrases:** "Stader exchange rate", "ETHx rate", "how much ETHx for my ETH", "Stader APY", "Stader stats"

**Usage:**
```
stader rates [--preview-amount <wei>] [--rpc-url <url>]
```

**Examples:**
```
stader rates
stader rates --preview-amount 1000000000000000000
```

**Output:** Current ETH→ETHx rate, total ETH staked, deposit limits, preview of ETHx received.

---

### positions — View your ETHx balance and withdrawals

**Trigger phrases:** "my Stader position", "ETHx balance", "pending Stader withdrawals", "Stader holdings"

**Usage:**
```
stader positions [--address <addr>] [--chain 1]
```

**Examples:**
```
stader positions
stader positions --address 0xabc...
```

**Output:** ETHx balance, ETH value, list of pending withdrawal requests with their status.

---

### stake — Deposit ETH to receive ETHx

**Trigger phrases:** "stake ETH with Stader", "buy ETHx", "deposit ETH Stader", "stake on Stader"

**IMPORTANT:** This is a write operation. **Ask user to confirm** the amount and receiver before executing.

**Usage:**
```
stader stake --amount <wei> [--receiver <addr>] [--chain 1] [--dry-run]
```

**Examples:**
```
stader stake --amount 100000000000000
stader stake --amount 1000000000000000000 --receiver 0xabc...
stader --dry-run stake --amount 100000000000000
```

**Notes:**
- `--amount` is in wei. Minimum: 100000000000000 (0.0001 ETH).
- Run with `--dry-run` first to preview calldata, then **ask user to confirm** before proceeding.
- After confirmation, executes: `onchainos wallet contract-call --chain 1 --to <StaderManager> --input-data <calldata> --amt <wei>`

---

### unstake — Request ETHx withdrawal (2-step: approve + requestWithdraw)

**Trigger phrases:** "unstake ETHx", "withdraw from Stader", "redeem ETHx", "Stader withdrawal"

**IMPORTANT:** This is a write operation. **Ask user to confirm** the ETHx amount and owner before executing.

**Usage:**
```
stader unstake --amount <ethx_wei> [--owner <addr>] [--chain 1] [--dry-run]
```

**Examples:**
```
stader unstake --amount 1000000000000000000
stader --dry-run unstake --amount 1000000000000000000
```

**Notes:**
- `--amount` is ETHx amount in wei.
- Two transactions are submitted: (1) ERC-20 `approve` ETHx to UserWithdrawManager, (2) `requestWithdraw`.
- Approval is skipped if existing allowance is sufficient.
- Withdrawal finalization takes ~3-10 days. Use `claim` once finalized.
- Run with `--dry-run` first to preview, then **ask user to confirm** before proceeding.
- After confirmation, executes `onchainos wallet contract-call` twice (approve + requestWithdraw).

---

### claim — Claim finalized ETH withdrawal

**Trigger phrases:** "claim Stader", "claim ETH withdrawal", "finalize Stader", "claim requestId"

**IMPORTANT:** This is a write operation. **Ask user to confirm** the request ID before executing.

**Usage:**
```
stader claim --request-id <id> [--chain 1] [--dry-run]
```

**Examples:**
```
stader claim --request-id 12345
stader --dry-run claim --request-id 12345
```

**Notes:**
- `--request-id` is the ID returned from the `unstake` command.
- The plugin checks if the request is finalized before attempting claim.
- If not finalized, returns an informative message without broadcasting.
- Run with `--dry-run` first to preview, then **ask user to confirm** before proceeding.
- After confirmation, executes: `onchainos wallet contract-call --chain 1 --to <UserWithdrawManager> --input-data <calldata>`

---

## Execution Flow for Write Operations

1. Run with `--dry-run` to preview the calldata
2. **Ask user to confirm** the operation details
3. Execute only after explicit user approval
4. Report transaction hash and outcome

---

## Important Notes

- **Withdrawal delay:** ETHx unstaking takes ~3-10 days to finalize. Users cannot speed this up.
- **Minimum deposit:** 0.0001 ETH (100000000000000 wei). The protocol rejects smaller amounts.
- **ETHx accumulates value:** Unlike rebasing tokens, ETHx price increases over time (1 ETHx > 1 ETH).
- **Chain:** Stader ETHx is only on Ethereum mainnet (chain 1), not on Base or other L2s.
