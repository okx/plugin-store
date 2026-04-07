---
name: frax-ether
description: "Frax Ether liquid staking protocol. Stake ETH to receive frxETH, then stake frxETH to earn yield as sfrxETH (ERC-4626 vault). Query rates, APR, and positions. Trigger phrases: stake ETH frax, stake frxETH, unstake sfrxETH, frax ether APR, frxETH yield, sfrxETH position, frax liquid staking. Chinese: 质押ETH到Frax, frxETH质押, sfrxETH收益, Frax以太坊质押"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Architecture

Frax Ether is a two-step liquid staking protocol on Ethereum mainnet:
1. ETH → frxETH via `frxETHMinter.submit()` (payable call)
2. frxETH → sfrxETH via ERC-4626 `deposit()` (yield-bearing vault)

- **Write ops** (stake, stake-frx, unstake) → after user confirmation, submits via `onchainos wallet contract-call`
- **Read ops** (rates, positions) → direct `eth_call` via Ethereum public RPC; no confirmation needed

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview calldata
2. **Ask user to confirm** before executing on-chain
3. Execute only after explicit user approval
4. Report transaction hash and link to etherscan.io

---

## Commands

### `stake` — Stake ETH to receive frxETH

Stake native ETH to receive liquid frxETH token via Frax's frxETHMinter contract.

**Parameters:**
- `--amount <float>` — Amount of ETH to stake (e.g. `0.001`)
- `--chain <id>` — Chain ID (default: `1`, Ethereum mainnet only)
- `--dry-run` — Preview calldata without broadcasting

**Example:**
```
frax-ether stake --amount 0.001 --chain 1
frax-ether stake --amount 0.001 --chain 1 --dry-run
```

**Execution:**
1. Run `--dry-run` to preview the transaction
2. **Ask user to confirm** before proceeding on-chain
3. Calls `frxETHMinter.submit(address)` with `--amt <wei>` via `onchainos wallet contract-call`
4. Returns txHash and link to etherscan.io

---

### `stake-frx` — Stake frxETH to receive yield-bearing sfrxETH

Deposit frxETH into the sfrxETH ERC-4626 vault to earn staking yield.

**Parameters:**
- `--amount <float>` — Amount of frxETH to stake (e.g. `0.001`)
- `--chain <id>` — Chain ID (default: `1`, Ethereum mainnet only)
- `--dry-run` — Preview calldata without broadcasting

**Example:**
```
frax-ether stake-frx --amount 0.001 --chain 1
frax-ether stake-frx --amount 0.001 --chain 1 --dry-run
```

**Execution (two-step):**
1. Run `--dry-run` to preview both approve and deposit calldata
2. **Ask user to confirm** before proceeding on-chain
3. Step 1: ERC-20 `approve(sfrxETH, amount)` on frxETH token via `onchainos wallet contract-call`
4. Step 2: ERC-4626 `deposit(assets, receiver)` on sfrxETH vault via `onchainos wallet contract-call`
5. Returns txHash for deposit and link to etherscan.io

---

### `unstake` — Redeem sfrxETH to receive frxETH

Redeem sfrxETH shares from the ERC-4626 vault to receive frxETH back.

**Parameters:**
- `--amount <float>` — Amount of sfrxETH to redeem (e.g. `0.001`)
- `--chain <id>` — Chain ID (default: `1`, Ethereum mainnet only)
- `--dry-run` — Preview calldata without broadcasting

**Example:**
```
frax-ether unstake --amount 0.001 --chain 1
frax-ether unstake --amount 0.001 --chain 1 --dry-run
```

**Execution:**
1. Run `--dry-run` to preview the transaction
2. **Ask user to confirm** before proceeding on-chain
3. Calls ERC-4626 `redeem(shares, receiver, owner)` via `onchainos wallet contract-call`
4. Returns txHash and received frxETH amount

---

### `rates` — Query sfrxETH APR and exchange rate

Get current sfrxETH staking yield, exchange rate, and total assets.

**Parameters:** None

**Example:**
```
frax-ether rates
```

**Execution:**
1. Fetches APR data from `https://api.frax.finance/v2/frxeth/summary/history?range=1d`
2. Calls `convertToAssets(1e18)` on sfrxETH for precise on-chain exchange rate

**Output fields:**
- `sfrxeth_apr_pct` — Annual percentage rate (%)
- `sfrxeth_per_frxeth` — How much frxETH 1 sfrxETH can be redeemed for
- `frxeth_per_eth_curve` — frxETH/ETH price on Curve
- `total_assets_frxeth` — Total frxETH in the sfrxETH vault
- `eth_price_usd` — Current ETH price in USD

---

### `positions` — Query frxETH and sfrxETH holdings

Get frxETH and sfrxETH balances with USD value for a wallet.

**Parameters:**
- `--address <addr>` — Wallet address to query (defaults to logged-in wallet)
- `--chain <id>` — Chain ID (default: `1`, Ethereum mainnet only)

**Example:**
```
frax-ether positions
frax-ether positions --address 0xabc...
```

**Execution:**
1. Calls `balanceOf(address)` on frxETH and sfrxETH contracts
2. Calls `convertToAssets(sfrxeth_balance)` to compute underlying frxETH value
3. Fetches ETH price for USD conversion

---

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|----------|---------|
| frxETHMinter | `0xbAFA44EFE7901E04E39Dad13167D089C559c1138` |
| frxETH token | `0x5E8422345238F34275888049021821E8E08CAa1f` |
| sfrxETH vault | `0xac3E018457B222d93114458476f3E3416Abbe38F` |
