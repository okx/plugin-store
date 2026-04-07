---
name: cian-yield-layer
description: >-
  Use when the user asks about CIAN Yield Layer, CIAN vaults, ylstETH, ylpumpBTC,
  'deposit stETH CIAN', 'stake ETH CIAN yield', 'CIAN recursive staking', 'CIAN restaking',
  'CIAN withdraw', 'request redeem CIAN', 'CIAN APY', 'CIAN TVL',
  'CIAN yield layer position', 'ylstETH balance', 'ylpumpBTC balance',
  'pumpBTC yield', 'CIAN pumpBTC vault', 'CIAN stETH vault',
  or mentions CIAN, CIAN Yield Layer, ylstETH, ylpumpBTC, CIAN DeFi.
  Covers: depositing ETH/stETH/wstETH/weETH/pumpBTC/WBTC into CIAN ERC4626 vaults,
  async withdrawal via requestRedeem (5-day process), balance and position queries on Ethereum.
  Do NOT use for general ETH staking unrelated to CIAN.
  Do NOT use for CIAN products other than Yield Layer.
license: MIT
metadata:
  author: ganlinux
  version: "0.1.0"
---

# CIAN Yield Layer Plugin

CIAN Yield Layer provides ERC4626 vaults on Ethereum Mainnet that amplify yield through
Recursive Staking (RS), Recursive Restaking (RR), and Hybrid Long-Short strategies.

**Available Vaults:**
- **ylstETH** (`0xB13aa2d0345b0439b064f26B82D8dCf3f508775d`) — stETH Yield Layer
  - Accepts: ETH, WETH, stETH, wstETH, eETH, weETH
  - Strategy: Recursive staking on Lido/EigenLayer (up to 6.5x-9x leverage)
- **ylpumpBTC** (`0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b`) — pumpBTC Yield Layer
  - Accepts: pumpBTC, WBTC
  - Strategy: BTC yield amplification

**Async Withdrawal Flow (CRITICAL — 5-day process):**
1. User calls `request-redeem` — shares are IMMEDIATELY transferred to the rebalancer (irreversible)
2. Protocol rebalancer processes the withdrawal over ~5 days
3. Assets are automatically sent to user's wallet (no further action needed)
4. Exit fee: up to 1.2% at operator's discretion

**IMPORTANT:** Standard ERC4626 `withdraw()` and `redeem()` are DISABLED.
`requestRedeem` is the ONLY supported withdrawal method.

**Write ops — after user confirmation, submits via `onchainos wallet contract-call`**

## Pre-flight Checks

Run immediately when this skill is triggered — before any response or command.

1. **Check onchainos**: `which onchainos` — if not found, tell user to install from https://web3.okx.com/zh-hans/onchainos/dev-docs/home/install-your-agentic-wallet
2. **Check binary**: `which cian-yield-layer` — if not found, install via `plugin-store install cian-yield-layer`
3. **Check wallet login**: `onchainos wallet status` — must show `loggedIn: true`; if not, run `onchainos wallet login`
4. **For write operations**: verify Ethereum (chain 1) wallet has sufficient ETH for gas

## Commands

### `cian-yield-layer vaults` — List All Vaults

**Triggers:** "CIAN vaults", "CIAN APY", "CIAN TVL", "CIAN yield rates", "list CIAN vaults"

```bash
cian-yield-layer vaults
```

**Output:** All vault names, addresses, APY breakdown (base + points + eco earn), TVL, accepted tokens.

---

### `cian-yield-layer balance` — Check Share Balances

**Triggers:** "CIAN balance", "ylstETH balance", "ylpumpBTC balance", "how much CIAN", "CIAN holdings"

```bash
cian-yield-layer balance
cian-yield-layer balance --wallet 0xYourAddress
```

**Parameters:**
- `--wallet <address>` — (optional) query a specific address instead of logged-in wallet

**Output:** Share balance per vault, estimated underlying asset value, current exchange rate.

---

### `cian-yield-layer positions` — View Full Positions

**Triggers:** "CIAN position", "CIAN holdings", "CIAN portfolio", "pending CIAN redeem"

```bash
cian-yield-layer positions
cian-yield-layer positions --wallet 0xYourAddress
```

**Parameters:**
- `--wallet <address>` — (optional) query a specific address

**Output:** Share balance, underlying value, USD value (from REST API), pending redeem shares.

---

### `cian-yield-layer deposit` — Deposit into Vault

**Triggers:** "deposit stETH CIAN", "stake ETH CIAN", "buy ylstETH", "deposit pumpBTC CIAN", "add CIAN position"

```bash
cian-yield-layer deposit --vault ylsteth --token stETH --amount 1.0
cian-yield-layer deposit --vault ylsteth --token ETH --amount 0.5
cian-yield-layer deposit --vault ylpumpbtc --token pumpBTC --amount 0.01
cian-yield-layer --dry-run deposit --vault ylsteth --token wstETH --amount 1.0
```

**Parameters:**
- `--vault <name>` — `ylsteth` or `ylpumpbtc` (or full vault address)
- `--token <symbol>` — token to deposit (ETH, WETH, stETH, wstETH, eETH, weETH, pumpBTC, WBTC)
- `--amount <value>` — amount in human-readable units (e.g. `1.5`) or raw wei
- `--from <address>` — (optional) override sender address

**Flow:**
1. Run `--dry-run` to preview, then **ask user to confirm** before proceeding
2. For ERC-20 tokens: check allowance, execute `approve` if needed (ask user to confirm)
3. Wait 3 seconds after approve to avoid nonce conflict
4. Execute `optionalDeposit(token, amount, receiver, address(0))`
5. For native ETH: no approve needed, ETH sent as msg.value

**Constraints:**
- ETH deposits use `address(0)` as token with msg.value; no approve needed
- ERC-20 deposits require prior approve to vault address
- If `exchangePrice` not updated in 3+ days, vault may reject deposits

**Output:** Approve TX hash (if needed), deposit TX hash, Etherscan links.

---

### `cian-yield-layer request-redeem` — Request Async Withdrawal

**Triggers:** "CIAN withdraw", "redeem ylstETH", "get stETH from CIAN", "CIAN exit", "request CIAN redeem"

```bash
cian-yield-layer request-redeem --vault ylsteth --shares 1.5 --token stETH
cian-yield-layer request-redeem --vault ylpumpbtc --shares 0.001 --token pumpBTC
cian-yield-layer --dry-run request-redeem --vault ylsteth --shares 1.0 --token stETH
```

**Parameters:**
- `--vault <name>` — `ylsteth` or `ylpumpbtc` (or full vault address)
- `--shares <amount>` — number of vault shares to redeem (e.g. `1.5`)
- `--token <symbol>` — token to receive back (e.g. stETH, WETH, pumpBTC, WBTC)
- `--from <address>` — (optional) override sender address

**Flow:**
1. Run `--dry-run` to preview, then **ask user to confirm** before proceeding
2. Check on-chain share balance to confirm sufficient shares
3. Preview estimated assets to be received
4. WARN user: this is irreversible, 5-day wait
5. Execute `requestRedeem(shares, token)` — selector `0x107703ab`

**Critical User Education:**
- Shares are IMMEDIATELY transferred to the rebalancer upon tx confirmation
- The withdrawal CANNOT be cancelled after submission
- No further user action needed; assets auto-arrive in ~5 days
- Exit fee up to 1.2% may be deducted by the protocol
- If assets don't arrive after 7+ days, contact CIAN support

**Output:** TX hash, Etherscan link, confirmation of shares transferred, expected timeline.

---

## Typical Workflows

### Workflow 1: Deposit ETH into stETH Yield Layer

```bash
# 1. Check current APY
cian-yield-layer vaults

# 2. Preview deposit
cian-yield-layer --dry-run deposit --vault ylsteth --token ETH --amount 1.0

# 3. Confirm and execute
cian-yield-layer deposit --vault ylsteth --token ETH --amount 1.0

# 4. Verify balance
cian-yield-layer balance
```

### Workflow 2: Deposit stETH

```bash
# 1. Preview (will show approve + deposit steps)
cian-yield-layer --dry-run deposit --vault ylsteth --token stETH --amount 1.0

# 2. Execute (approve first, then deposit)
cian-yield-layer deposit --vault ylsteth --token stETH --amount 1.0
```

### Workflow 3: Withdraw (5-day async process)

```bash
# 1. Check shares
cian-yield-layer balance

# 2. Preview redeem
cian-yield-layer --dry-run request-redeem --vault ylsteth --shares 1.0 --token stETH

# 3. Submit request (IRREVERSIBLE)
cian-yield-layer request-redeem --vault ylsteth --shares 1.0 --token stETH

# 4. Track pending status
cian-yield-layer positions
# Assets arrive ~5 days later automatically
```

## Contract Addresses (Ethereum Mainnet)

| Contract | Address |
|----------|---------|
| ylstETH Vault (Proxy) | `0xB13aa2d0345b0439b064f26B82D8dCf3f508775d` |
| ylpumpBTC Vault (Proxy) | `0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b` |
| stETH | `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84` |
| wstETH | `0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0` |
| weETH | `0xCd5fE23C85820F7B72D0926FC9b05b43E359b7ee` |
| pumpBTC | `0xF469fBD2abcd6B9de8E169d128226C0Fc90a6Ff9` |
| WBTC | `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599` |

## Key Function Selectors

| Function | Selector |
|----------|---------|
| `optionalDeposit(address,uint256,address,address)` | `0x32507a5f` |
| `requestRedeem(uint256,address)` | `0x107703ab` |
| `balanceOf(address)` | `0x70a08231` |
| `convertToAssets(uint256)` | `0x07a2d13a` |
| `exchangePrice()` | `0x9e65741e` |
| `approve(address,uint256)` | `0x095ea7b3` |
