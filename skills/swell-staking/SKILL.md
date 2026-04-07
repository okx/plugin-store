---
name: swell-staking
description: "Stake ETH with Swell Network to receive swETH (liquid staking) or rswETH (liquid restaking via EigenLayer) on Ethereum mainnet. Query exchange rates and positions. Trigger phrases: stake ETH swell, buy swETH, restake ETH EigenLayer, check swETH balance, rswETH rate, swell staking positions. Chinese: Swell质押ETH, 获取swETH, EigenLayer再质押, 查询swETH余额"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Swell Network Staking Plugin

## Overview

This plugin enables interaction with Swell Network on Ethereum mainnet (chain ID 1). Users can:
- Stake ETH to receive **swETH** (liquid staking token, accrues validator rewards)
- Restake ETH to receive **rswETH** (liquid restaking token, earns both validator + EigenLayer rewards)
- Query current exchange rates for swETH and rswETH
- View swETH and rswETH holdings for any address

**Key facts:**
- Both swETH and rswETH are ERC-20 tokens that appreciate in value vs ETH over time (non-rebasing)
- Only Ethereum mainnet (chain 1) is supported
- Unstaking involves a 1–7 day queue period and is handled via the Swell app (not this plugin)
- All write operations require user confirmation before submission

## Architecture

- Read ops (rates, positions) → direct `eth_call` via public Ethereum RPC; no wallet required
- Write ops (stake, restake) → after user confirmation, submits via `onchainos wallet contract-call`

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|---|---|
| swETH (Liquid Staking) | `0xf951E335afb289353dc249e82926178EaC7DEd78` |
| rswETH (Liquid Restaking) | `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0` |

---

## Commands

### `rates` — Get Exchange Rates

Query the current swETH and rswETH exchange rates against ETH.

**Usage:**
```
swell-staking rates [--chain 1]
```

**Steps:**
1. eth_call `swETHToETHRate()` on swETH contract
2. eth_call `ethToSwETHRate()` on swETH contract
3. eth_call `rswETHToETHRate()` on rswETH contract
4. eth_call `ethToRswETHRate()` on rswETH contract
5. Display all rates in human-readable format

**No wallet required. No onchainos write call needed.**

**Example output:**
```json
{
  "swETH": { "ETH_per_swETH": "1.119...", "swETH_per_ETH": "0.893..." },
  "rswETH": { "ETH_per_rswETH": "1.069...", "rswETH_per_ETH": "0.935..." }
}
```

---

### `positions` — View Holdings

Query swETH and rswETH balances for an address.

**Usage:**
```
swell-staking positions [--address <ADDR>] [--chain 1]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--address` | No | Address to query (resolved from onchainos if omitted) |

**Steps:**
1. Resolve address (from arg or onchainos)
2. eth_call `balanceOf(address)` on swETH contract
3. eth_call `balanceOf(address)` on rswETH contract
4. Fetch exchange rates to compute ETH-denominated values
5. Display positions

**No onchainos write call needed.**

---

### `stake` — Stake ETH for swETH

Deposit ETH into the Swell liquid staking contract to receive swETH.

**Usage:**
```
swell-staking stake --amount <ETH_AMOUNT> [--from <ADDR>] [--dry-run] [--chain 1]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | ETH amount to stake (e.g. `0.001`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Parse and validate amount
2. If `--dry-run`, return simulated response immediately
3. Resolve wallet address
4. Fetch current `ethToSwETHRate()` to show expected swETH output
5. Display: amount, expected swETH, contract address
6. **Ask user to confirm** before submitting the transaction
7. Execute: `onchainos wallet contract-call --chain 1 --to 0xf951E335afb289353dc249e82926178EaC7DEd78 --input-data 0xd0e30db0 --amt <WEI> --force`
8. Return txHash and Etherscan link

**Calldata structure:** `0xd0e30db0` (deposit() selector only — no parameters, ETH value sent via --amt)

---

### `restake` — Restake ETH for rswETH (EigenLayer)

Deposit ETH into the Swell liquid restaking contract to receive rswETH, earning both validator and EigenLayer restaking rewards.

**Usage:**
```
swell-staking restake --amount <ETH_AMOUNT> [--from <ADDR>] [--dry-run] [--chain 1]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | ETH amount to restake (e.g. `0.001`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Parse and validate amount
2. If `--dry-run`, return simulated response immediately
3. Resolve wallet address
4. Fetch current `ethToRswETHRate()` to show expected rswETH output
5. Display: amount, expected rswETH, contract address, EigenLayer context
6. **Ask user to confirm** before submitting the transaction
7. Execute: `onchainos wallet contract-call --chain 1 --to 0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0 --input-data 0xd0e30db0 --amt <WEI> --force`
8. Return txHash and Etherscan link

**Calldata structure:** `0xd0e30db0` (deposit() selector only — no parameters, ETH value sent via --amt)

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "Cannot resolve wallet address" | Not logged in to onchainos | Run `onchainos wallet login` |
| "Stake amount must be greater than 0" | Zero or invalid amount | Provide a positive ETH amount |
| "Unsupported chain_id" | Non-Ethereum chain specified | Swell only supports chain 1 (Ethereum mainnet) |
| eth_call RPC error | RPC rate limit or network issue | Retry; check https://ethereum.publicnode.com status |

## Notes

- **Unstaking:** swETH and rswETH can be unstaked via the Swell app (https://app.swellnetwork.io). The process takes 1–7 days and generates a swEXIT NFT. This plugin does not implement unstaking.
- **Rate appreciation:** Unlike rebasing tokens (e.g. stETH), swETH and rswETH appreciate in price vs ETH as rewards accumulate.
- **EigenLayer:** rswETH holders additionally earn EigenLayer AVS restaking rewards on top of base validator yield.

## Skill Routing

- For Lido staking (stETH) → use the `lido` skill
- For wallet balance → use `onchainos wallet balance --chain 1`
- For Swell unstaking → direct users to https://app.swellnetwork.io
