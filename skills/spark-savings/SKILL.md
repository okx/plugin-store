---
name: spark-savings
description: "Spark Savings — earn Sky Savings Rate (SSR) on USDS/DAI. Trigger phrases: spark savings, deposit to spark, earn savings rate, sUSDS APY, sDAI rate, sky savings, MakerDAO savings, DSR rate, deposit USDS, stake USDS for yield, withdraw sUSDS, spark savings rate, 存入Spark储蓄, 查询储蓄利率, Spark储蓄年化, 存USDS赚利息"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Spark Savings Skill

## Overview

Spark Savings (by SparkFi / MakerDAO/Sky ecosystem) lets users deposit USDS (or DAI on Ethereum) into savings vaults to earn the **Sky Savings Rate (SSR)** — currently ~3.75% APY. The vault token is **sUSDS** (Savings USDS).

On **Ethereum**: sUSDS and sDAI are ERC-4626 vaults — deposit directly.
On **Base, Arbitrum, Optimism**: sUSDS is a bridged token; deposits/withdrawals go through the **Spark PSM3** contract (swaps USDS ↔ sUSDS).

**Supported chains:**

| Chain | Chain ID | Mechanism |
|-------|----------|-----------|
| Ethereum Mainnet | 1 | ERC-4626 direct |
| Base | 8453 (default) | PSM3 swap |
| Arbitrum One | 42161 | PSM3 swap |
| Optimism | 10 | PSM3 swap |

---

## Pre-flight Checks

Before any command:
1. **Binary installed**: `spark-savings --version`
2. **Wallet connected**: `onchainos wallet status`
3. **Chain supported**: must be 1, 8453, 42161, or 10

---

## Command Routing

| User Intent | Command |
|-------------|---------|
| Check current APY / savings rate | `spark-savings --chain <ID> apy` |
| Check my sUSDS balance | `spark-savings --chain <ID> balance` |
| Deposit USDS to earn savings | `spark-savings --chain <ID> --dry-run deposit --amount <N>` |
| Withdraw sUSDS back to USDS | `spark-savings --chain <ID> --dry-run withdraw --amount <N>` |
| Withdraw all sUSDS | `spark-savings --chain <ID> --dry-run withdraw --all` |
| Show market info / TVL | `spark-savings --chain <ID> markets` |

**Global flags:**
- `--chain <ID>` — target chain (default: 8453 Base)
- `--from <ADDRESS>` — override wallet address
- `--dry-run` — simulate without broadcasting

---

## Commands

### apy — Current savings rate

**Trigger phrases:** "spark savings APY", "sUSDS rate", "sky savings rate", "DSR rate", "what's the spark yield", "储蓄利率", "Spark年化"

```bash
spark-savings --chain 8453 apy
spark-savings --chain 1 apy
```

**Output includes:**
- SSR APY (Sky Savings Rate for sUSDS)
- DSR APY (DAI Savings Rate for sDAI)
- sUSDS/USDS conversion rate

---

### balance — Check savings balance

**Trigger phrases:** "my spark savings", "sUSDS balance", "how much sUSDS do I have", "我的Spark储蓄余额"

```bash
spark-savings --chain 8453 balance
spark-savings --chain 1 balance --from 0xYourAddress
```

**Output includes:**
- sUSDS balance (shares)
- USDS equivalent value
- USDS wallet balance
- (Ethereum only) sDAI balance and DAI equivalent

---

### deposit — Deposit USDS to earn savings

**Trigger phrases:** "deposit to spark", "earn savings on USDS", "stake USDS in spark", "存入Spark储蓄", "把USDS存入spark"

**IMPORTANT: Always show dry-run first and ask user to confirm before executing.**

```bash
# Step 1: dry run
spark-savings --chain 8453 --dry-run deposit --amount 10.0
# Step 2: execute after user confirms
spark-savings --chain 8453 deposit --amount 10.0
```

**Flow on L2 (Base/Arbitrum/Optimism):**
1. `USDS.approve(PSM3, amount)`
2. `PSM3.swapExactIn(USDS, sUSDS, amount, 0, receiver, 0)`

**Flow on Ethereum:**
1. `USDS.approve(sUSDS, amount)`
2. `sUSDS.deposit(amount, receiver)`

**Output:**
```json
{
  "ok": true,
  "amountUSDS": "10.000000",
  "estimatedSUSDS": "9.156030",
  "approveTxHash": "0x...",
  "depositTxHash": "0x..."
}
```

---

### withdraw — Withdraw sUSDS to USDS

**Trigger phrases:** "withdraw from spark", "redeem sUSDS", "取出Spark储蓄", "赎回sUSDS"

**IMPORTANT: Always show dry-run first and ask user to confirm before executing.**

```bash
# Withdraw specific amount of sUSDS
spark-savings --chain 8453 --dry-run withdraw --amount 9.0
# Withdraw all sUSDS
spark-savings --chain 8453 --dry-run withdraw --all
# Execute after confirmation
spark-savings --chain 8453 withdraw --amount 9.0
```

**Flow on L2:**
1. `sUSDS.approve(PSM3, shares)`
2. `PSM3.swapExactIn(sUSDS, USDS, shares, 0, receiver, 0)`

**Flow on Ethereum:**
1. `sUSDS.redeem(shares, receiver, owner)`

---

### markets — Savings market info

**Trigger phrases:** "spark market", "sUSDS TVL", "spark savings stats", "储蓄市场数据"

```bash
spark-savings --chain 8453 markets
spark-savings --chain 1 markets
```

**Output includes:**
- SSR and DSR APY
- PSM3 / vault TVL
- sUSDS total supply and conversion rate
- Contract addresses

---

## Safety Rules

1. **Always dry-run first** for deposit/withdraw: show simulated commands and expected output
2. **Ask user to confirm** before broadcasting any write transaction
3. **Check balance** before withdraw — show current sUSDS balance in dry-run output
4. **No slippage protection** in plugin (minAmountOut = 0) — inform user for large amounts
5. **Reserve gas**: warn user if ETH balance is below 0.001 ETH on the target chain

---

## Contract Addresses Reference

### Base (8453) — Default
| Name | Address |
|------|---------|
| sUSDS | `0x5875eEE11Cf8398102FdAd704C9E96607675467a` |
| USDS | `0x820C137fa70C8691f0e44Dc420a5e53c168921Dc` |
| PSM3 | `0x1601843c5E9bC251A3272907010AFa41Fa18347E` |

### Ethereum (1)
| Name | Address |
|------|---------|
| sUSDS | `0xa3931d71877C0E7a3148CB7Eb4463524FEc27fbD` |
| sDAI | `0x83F20F44975D03b1b09e64809B757c47f942BEeA` |
| USDS | `0xdC035D45d973E3EC169d2276DDab16f1e407384F` |
| DAI | `0x6B175474E89094C44Da98b954EedeAC495271d0F` |

### Arbitrum (42161)
| Name | Address |
|------|---------|
| sUSDS | `0xdDb46999F8891663a8F2828d25298f70416d7610` |
| USDS | `0x6491c05a82219b8d1479057361ff1654749b876b` |
| PSM3 | `0x2B05F8e1cACC6974fD79A673a341Fe1f58d27266` |

### Optimism (10)
| Name | Address |
|------|---------|
| sUSDS | `0xb5B2dc7fd34C249F4be7fB1fCea07950784229e0` |
| USDS | `0x4F13a96EC5C4Cf34e442b46Bbd98a0791F20edC3` |
| PSM3 | `0xe0F9978b907853F354d79188A3dEfbD41978af62` |

---

## Troubleshooting

| Error | Solution |
|-------|----------|
| `Could not resolve wallet` | Run `onchainos wallet login` |
| `Insufficient sUSDS balance` | Check balance with `balance` command first |
| `eth_call RPC error` | RPC rate-limited; retry |
| `Unsupported chain ID` | Use 1, 8453, 42161, or 10 |
