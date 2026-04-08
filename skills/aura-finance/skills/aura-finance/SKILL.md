---
name: aura-finance
description: "Deposit Balancer LP tokens (BPT) into Aura Finance for boosted BAL and AURA rewards on Ethereum. Trigger phrases: aura finance deposit, aura bpt staking, aura rewards, claim aura, aura finance pools, lock aura, vlAURA, aura finance position, balancer boosted yield aura."
version: "0.1.0"
author: "skylavis-sky"
tags:
  - yield
  - balancer
  - aura
  - vlAURA
  - ethereum
  - defi
---

## Architecture

Aura Finance is to Balancer what Convex is to Curve - it deposits Balancer Pool Tokens (BPT) into gauges and distributes boosted BAL + AURA rewards to depositors.

This plugin supports:
- Listing Aura-supported Balancer pools with pool IDs and TVL
- Checking user positions (staked BPT, pending BAL/AURA rewards, vlAURA balance)
- Depositing BPT into Aura Booster pools (2-tx: approve + deposit)
- Withdrawing staked BPT from BaseRewardPool
- Claiming BAL + AURA rewards from BaseRewardPool
- Locking AURA as vlAURA (16-week lock) for governance and rewards
- Processing expired vlAURA locks to retrieve AURA

Write ops (deposit, withdraw, claim-rewards, lock-aura, unlock-aura) require user confirmation before submitting via `onchainos wallet contract-call`.
Read ops (get-pools, get-position) use direct eth_call via public RPC; no confirmation needed.
Chain: Ethereum mainnet (chain ID 1) only.

**Do NOT use for:**
- Balancer LP provision (use Balancer directly)
- Convex Finance (use convex skill)
- auraBAL conversion (irreversible - use Aura web UI)
- chains other than Ethereum mainnet

**BPT Prerequisite:** You must already hold Balancer Pool Tokens (BPT) for the target pool before depositing into Aura. BPT is obtained by adding liquidity on Balancer first. If your BPT balance is 0, the deposit command will error with instructions.

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview calldata
2. **Ask user to confirm** before executing on-chain (required for all write ops - E106)
3. Execute only after explicit user approval
4. Report transaction hash with Etherscan link

---

## Commands

### get-pools - List Aura Finance Pools

Lists Aura-supported Balancer pools from on-chain Booster data, enriched with Balancer API TVL where available.

```
aura-finance get-pools [--limit <n>] [--chain 1]
```

**Parameters:**
- `--limit` (optional, default 10): Number of pools to return
- `--chain` (optional, default 1): Chain ID

**Example:** "Show me the top Aura Finance pools"
```
aura-finance get-pools --limit 10
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "total_pool_count": 170,
    "scanned": 50,
    "shown": 10,
    "pools": [
      {
        "aura_pid": 29,
        "lp_token": "0x32296969...",
        "crv_rewards": "0x...",
        "tokens": ["wstETH", "WETH"],
        "tvl_usd": "$120000000",
        "shutdown": false
      }
    ]
  }
}
```

---

### get-position - Check Aura Position

Queries vlAURA locked balance, liquid AURA/BAL balances, and optionally a specific pool's staked BPT and pending rewards.

```
aura-finance get-position [--pool-id <pid>] [--address <wallet>] [--chain 1]
```

**Parameters:**
- `--pool-id` (optional): Aura pool PID to check staked BPT and pending BAL rewards
- `--address` (optional): Wallet to query (defaults to onchainos logged-in wallet)

**Example:** "What are my Aura Finance positions?"
```
aura-finance get-position --pool-id 29 --chain 1
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "wallet": "0x...",
    "vlAURA_locked": {
      "balance": "500.0",
      "note": "16-week lock period"
    },
    "liquid_balances": {
      "AURA": "10.0",
      "BAL": "2.5"
    },
    "pool_position": {
      "pool_id": 29,
      "staked_bpt": "1.5",
      "pending_bal_rewards": "0.34"
    }
  }
}
```

---

### deposit - Deposit BPT into Aura Pool

Approves BPT and deposits into an Aura Booster pool with `_stake=true` to start reward accrual immediately.
This is a 2-transaction flow: ERC-20 approve, then deposit (15s delay between them).

**Ask user to confirm** the approve and deposit transactions separately.

```
aura-finance deposit --pool-id <pid> --amount <amount> [--from <wallet>] [--chain 1] [--dry-run]
```

**Parameters:**
- `--pool-id` (required): Aura pool PID (use get-pools to find the right PID)
- `--amount` (required): Amount of BPT to deposit (e.g., 1.5)
- `--from` (optional): Wallet address override
- `--dry-run` (optional): Preview calldata without broadcasting

**Prerequisite:** You must hold BPT for the target pool. If BPT balance is 0, deposit will fail with instructions to add liquidity on Balancer first.

**Execution steps:**
1. Fetch BPT address from Booster.poolInfo(pid)
2. Check BPT balance (error if 0 - need to add Balancer liquidity first)
3. If needed: approve BPT spending -> `onchainos wallet contract-call` -> **ask user to confirm**
4. Wait ~15s for approval to confirm
5. Deposit BPT -> `onchainos wallet contract-call` -> **ask user to confirm**

**Example:** "Deposit 1.5 BPT into Aura pool 29"
```
aura-finance deposit --pool-id 29 --amount 1.5 --chain 1
```

---

### withdraw - Withdraw Staked BPT

Withdraws staked BPT from an Aura BaseRewardPool using `withdrawAndUnwrap`. Rewards are NOT claimed automatically (use claim-rewards separately).

**Ask user to confirm** before submitting the withdrawal transaction.

```
aura-finance withdraw --pool-id <pid> --amount <amount> [--from <wallet>] [--chain 1] [--dry-run]
```

**Parameters:**
- `--pool-id` (required): Aura pool PID
- `--amount` (required): Amount of BPT to withdraw
- `--from` (optional): Wallet address override
- `--dry-run` (optional): Preview calldata

**Example:** "Withdraw 1 BPT from Aura pool 29"
```
aura-finance withdraw --pool-id 29 --amount 1.0 --chain 1
```

---

### claim-rewards - Claim BAL + AURA Rewards

Claims pending BAL and AURA rewards from a pool's BaseRewardPool. Uses `getReward(address, bool)` with `_claimExtras=true` to claim both BAL and AURA from all reward distributors.

**Ask user to confirm** before submitting.

```
aura-finance claim-rewards --pool-id <pid> [--claim-extras] [--from <wallet>] [--chain 1] [--dry-run]
```

**Parameters:**
- `--pool-id` (required): Aura pool PID
- `--claim-extras` (optional, default true): Claim extra rewards (AURA and additional tokens)
- `--from` (optional): Wallet address override
- `--dry-run` (optional): Preview calldata

**Example:** "Claim my rewards from Aura pool 29"
```
aura-finance claim-rewards --pool-id 29 --chain 1
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "action": "claim-rewards",
    "pool_id": 29,
    "pending_bal_rewards": "0.34",
    "txHash": "0x..."
  }
}
```

---

### lock-aura - Lock AURA as vlAURA

**WARNING: LOCKING AURA AS vlAURA IS IRREVERSIBLE FOR 16 WEEKS.**
**You CANNOT withdraw early. AURA will be locked until the epoch expires.**

Locks AURA tokens in AuraLocker to receive vlAURA voting power and earn protocol rewards.
This is a 2-transaction flow: ERC-20 approve AURA, then lock (15s delay between them).

**Ask user to confirm** both the approve and the lock transactions. Always display the 16-week irreversibility warning before confirming.

```
aura-finance lock-aura --amount <amount> [--from <wallet>] [--chain 1] [--dry-run]
```

**Parameters:**
- `--amount` (required): Amount of AURA to lock (e.g., 100)
- `--from` (optional): Wallet address override
- `--dry-run` (optional): Preview calldata

**Execution steps:**
1. Check AURA balance
2. If needed: approve AURA -> `onchainos wallet contract-call` -> **ask user to confirm** (warn: 16-week lock)
3. Wait ~15s for approval to confirm
4. Lock AURA -> `onchainos wallet contract-call` -> **ask user to confirm** (display WARNING: 16-week irreversible lock)

**Example:** "Lock 500 AURA tokens as vlAURA"
```
aura-finance lock-aura --amount 500 --chain 1
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "action": "lock-aura",
    "amount": 500,
    "lock_period": "16 weeks",
    "txHash": "0x...",
    "WARNING": "AURA is now locked as vlAURA for 16 weeks. Use unlock-aura after lock expires."
  }
}
```

---

### unlock-aura - Process Expired vlAURA Locks

Processes expired vlAURA locks to withdraw AURA back to your wallet (or re-lock for another 16 weeks).
Will revert if there are no expired locks.

**Ask user to confirm** before submitting.

```
aura-finance unlock-aura [--relock] [--from <wallet>] [--chain 1] [--dry-run]
```

**Parameters:**
- `--relock` (optional, default false): Re-lock AURA for another 16 weeks instead of withdrawing
- `--from` (optional): Wallet address override
- `--dry-run` (optional): Preview calldata

**Example:** "Unlock my expired vlAURA"
```
aura-finance unlock-aura --chain 1
```

---

## Key Contracts (Ethereum Mainnet)

| Contract | Address |
|----------|---------|
| Booster | 0xA57b8d98dAE62B26Ec3bcC4a365338157060B234 |
| AURA token | 0xC0c293ce456fF0ED870ADd98a0828Dd4d2903DBF |
| AuraLocker (vlAURA) | 0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC |
| auraBAL token | 0x616e8BfA43F920657B3497DBf40D6b1A02D4608d |
| BAL token | 0xba100000625a3754423978a60c9317c58a424e3D |

## Gotchas

- BPT is acquired on Balancer (outside this plugin's scope). If you have 0 BPT, deposit will fail with a helpful error.
- BaseRewardPool addresses vary per pool - always fetched dynamically from Booster.poolInfo(pid). Never hardcoded.
- auraBAL conversion (BAL to auraBAL) is one-way at the contract level. Use the Aura web UI for this.
- vlAURA 16-week lock is irreversible until expiry. Always confirm with user before lock-aura.
- `onchainos wallet balance --chain 1 --output json` is NOT supported. This plugin uses the correct pattern.
