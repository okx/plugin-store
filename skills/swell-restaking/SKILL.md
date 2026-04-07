---
name: swell-restaking
description: "Stake ETH on Swell Network to receive rswETH (EigenLayer liquid restaking token) on Ethereum mainnet. Query rswETH exchange rates, check positions, and deposit ETH to earn both validator rewards and EigenLayer restaking rewards. Trigger phrases: restake ETH swell, buy rswETH, swell eigenlayer restaking, check rswETH balance, rswETH rate, swell restaking positions, stake for rswETH, deposit rswETH, swell liquid restaking"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

# Swell Restaking Plugin

## Overview

This plugin enables interaction with Swell Network's liquid restaking protocol on Ethereum mainnet (chain ID 1). Users can:

- Stake ETH to receive **rswETH** (liquid restaking token, earns validator + EigenLayer restaking rewards)
- Query the current rswETH exchange rate and pool statistics
- View rswETH holdings and ETH-equivalent value for any address

**Key facts:**
- rswETH is an ERC-20 token that appreciates in value vs ETH over time (non-rebasing)
- Only Ethereum mainnet (chain 1) is supported
- Minimum practical stake: 0.00005 ETH (GUARDRAILS test limit)
- Unstaking involves a 1-12 day queue period handled via the Swell app (not this plugin)
- All write operations require user confirmation before submission

**rswETH vs swETH:**
- swETH = Swell liquid staking (validator rewards only) - use the `swell-staking` skill
- rswETH = Swell liquid restaking (validator rewards + EigenLayer AVS rewards)

## Architecture

- Read ops (get-rates, get-positions) - direct `eth_call` via public Ethereum RPC; no wallet required
- Write ops (stake) - after user confirmation, submits via `onchainos wallet contract-call`

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|---|---|
| rswETH (Liquid Restaking) | `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0` |

---

## Commands

### `get-rates` - Get rswETH Exchange Rates

Query the current rswETH exchange rate against ETH and pool statistics.

**Usage:**
```
swell-restaking get-rates [--chain 1]
```

**Steps:**
1. eth_call `rswETHToETHRate()` on rswETH contract
2. eth_call `ethToRswETHRate()` on rswETH contract
3. eth_call `totalETHDeposited()` for TVL
4. eth_call `totalSupply()` for circulating supply
5. Display all rates in human-readable format

**No wallet required. No onchainos write call needed.**

**Example output:**
```json
{
  "rswETH": {
    "rswETH_per_ETH": "0.93543...",
    "ETH_per_rswETH": "1.06902...",
    "total_eth_deposited": "147006.13...",
    "total_supply": "137510.42..."
  }
}
```

---

### `get-positions` - View rswETH Holdings

Query rswETH balance and ETH-equivalent value for a wallet address.

**Usage:**
```
swell-restaking get-positions [--address <ADDR>] [--chain 1]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--address` | No | Address to query (resolved from onchainos if omitted) |

**Steps:**
1. Resolve address (from arg or onchainos wallet balance)
2. eth_call `balanceOf(address)` on rswETH contract
3. eth_call `rswETHToETHRate()` to compute ETH-denominated value
4. Display position with ETH equivalent

**No onchainos write call needed.**

---

### `stake` - Stake ETH for rswETH (EigenLayer)

Deposit ETH into the Swell liquid restaking contract to receive rswETH. rswETH earns both Ethereum validator rewards and EigenLayer AVS restaking rewards.

**Usage:**
```
swell-restaking stake --amount <ETH_AMOUNT> [--from <ADDR>] [--dry-run] [--chain 1]
```

**Parameters:**
| Parameter | Required | Description |
|---|---|---|
| `--amount` | Yes | ETH amount to stake (e.g. `0.00005`) |
| `--from` | No | Wallet address (resolved from onchainos if omitted) |
| `--dry-run` | No | Show calldata without broadcasting |

**Steps:**
1. Parse and validate amount (must be > 0)
2. If `--dry-run`, return simulated response immediately with calldata
3. Resolve wallet address
4. Fetch current `ethToRswETHRate()` to show expected rswETH output
5. Display: amount, expected rswETH, contract address, EigenLayer context
6. **Ask user to confirm** before submitting the transaction
7. Execute: `onchainos wallet contract-call --chain 1 --to 0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0 --input-data 0xd0e30db0 --amt <WEI> --force`
8. Return txHash and Etherscan link

**Calldata structure:** `0xd0e30db0` (deposit() selector only - no parameters, ETH value sent via --amt)

---

## Error Handling

| Error | Cause | Resolution |
|---|---|---|
| "Cannot resolve wallet address" | Not logged in to onchainos | Run `onchainos wallet login` |
| "Stake amount must be greater than 0" | Zero or invalid amount | Provide a positive ETH amount |
| "Warning: only supports chain 1" | Non-Ethereum chain specified | Swell Restaking only supports Ethereum mainnet |
| eth_call RPC error | RPC rate limit or network issue | Retry; check https://ethereum.publicnode.com |

## Notes

- **Unstaking rswETH:** rswETH can be unstaked via the Swell app at https://app.swellnetwork.io. The process takes 1-12 days (depending on EigenLayer queue depth) and generates an NFT withdrawal request. This plugin does not implement unstaking.
- **Rate appreciation:** rswETH appreciates in price vs ETH as rewards accumulate. Unlike rebasing tokens (e.g. stETH), the token count stays the same but each token is worth more ETH.
- **EigenLayer:** rswETH holders earn EigenLayer AVS restaking rewards on top of base validator yield. APY is typically higher than swETH.
- **DeFi utility:** rswETH can be used as collateral in Aave V3, Morpho, and other DeFi protocols.

## Skill Routing

- For Swell liquid staking (swETH) - use the `swell-staking` skill
- For Lido staking (stETH) - use the `lido` skill
- For wallet balance - use `onchainos wallet balance --chain 1`
- For rswETH unstaking - direct users to https://app.swellnetwork.io
