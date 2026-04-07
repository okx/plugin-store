---
name: lifi
version: "0.1.0"
description: "LI.FI/Jumper cross-chain bridge and swap aggregator for EVM chains"
---

# LI.FI / Jumper Skill

LI.FI is a cross-chain bridge and DEX aggregator that routes transactions through the best available bridges (Across, Stargate, Hop, etc.) and DEXes. It supports 79+ EVM chains including Ethereum, Arbitrum, Base, Polygon, Optimism, and BSC.

## Architecture

- Read ops (get-chains, get-tokens, get-quote, get-status, get-tools) call the LI.FI REST API directly.
- Write ops (swap) fetch a quote from LI.FI API, then after user confirmation, submit via `onchainos wallet contract-call` to the LiFiDiamond contract (`0x1231DEB6f5749EF6cE6943a275A1D3E7486F4EaE`).

---

## Commands

### get-chains - List supported chains

**Triggers:** "what chains does LI.FI support", "show LI.FI chains", "list supported networks"

**Usage:**
```
lifi get-chains
```

**Output:** List of mainnet EVM chains with IDs, names, and diamond contract addresses.

---

### get-tokens - List tokens on a chain

**Triggers:** "show USDC on Base", "what tokens are on Arbitrum in LI.FI", "list tokens for chain 8453"

**Usage:**
```
lifi get-tokens --chains <chain_ids> [--symbol <SYMBOL>]
```

**Parameters:**
- `--chains` — comma-separated chain IDs (default: 8453)
- `--symbol` — filter by token symbol (optional)

**Examples:**
```
lifi get-tokens --chains 8453 --symbol USDC
lifi get-tokens --chains 1,8453,42161
```

---

### get-quote - Get a bridge/swap quote

**Triggers:** "quote bridge USDC from Base to Arbitrum", "how much will I receive bridging 100 USDT to Ethereum", "get LI.FI quote for swapping ETH to USDC"

**Usage:**
```
lifi get-quote --from-chain <ID> --to-chain <ID> --from-token <TOKEN> --to-token <TOKEN> --amount <RAW_AMOUNT> [--slippage <SLIPPAGE>]
```

**Parameters:**
- `--from-chain` — source chain ID (default: --chain flag)
- `--to-chain` — destination chain ID
- `--from-token` — source token symbol or address
- `--to-token` — destination token symbol or address
- `--amount` — amount in raw token units (e.g., 10000000 = 10 USDT with 6 decimals)
- `--slippage` — slippage tolerance, default 0.005 (0.5%)

**Example:**
```
lifi get-quote --from-chain 8453 --to-chain 42161 --from-token USDC --to-token USDC --amount 5000000
```

---

### get-status - Check transfer status

**Triggers:** "check my LI.FI transfer status", "status of bridge tx 0xabc", "did my cross-chain transfer complete"

**Usage:**
```
lifi get-status --tx-hash <HASH> [--from-chain <ID>] [--to-chain <ID>]
```

**Parameters:**
- `--tx-hash` — source chain transaction hash
- `--from-chain` — source chain ID (optional)
- `--to-chain` — destination chain ID (optional)

**Output:** status (DONE/PENDING/FAILED), source and destination tx hashes, LI.FI explorer link.

---

### get-tools - List available bridges and DEXes

**Triggers:** "what bridges does LI.FI use", "show LI.FI exchanges", "list LI.FI tools"

**Usage:**
```
lifi get-tools [--chains <chain_ids>]
```

---

### swap - Execute a cross-chain swap or bridge

**Triggers:** "bridge USDC from Base to Arbitrum", "swap ETH to USDC on Base via LI.FI", "send 10 USDT from Ethereum to Base"

**Usage:**
```
lifi [--chain <SRC_CHAIN_ID>] swap --to-chain <ID> --from-token <TOKEN> --to-token <TOKEN> --amount <RAW_AMOUNT> [--slippage <SLIPPAGE>] [--from <WALLET>] [--dry-run]
```

**Parameters:**
- `--chain` / `--from-chain` — source chain ID (default: 8453 Base)
- `--to-chain` — destination chain ID
- `--from-token` — source token symbol or address
- `--to-token` — destination token symbol or address
- `--amount` — amount in raw token units (e.g., 5000000 = 5 USDC)
- `--slippage` — slippage tolerance (default 0.005 = 0.5%)
- `--from` — sender wallet address (resolved from onchainos if omitted)
- `--dry-run` — preview the transaction without broadcasting

**Flow:**
1. Fetches best route from LI.FI API
2. Displays quote: source/destination amounts, fees, bridge used
3. Ask user to confirm before proceeding with the on-chain transaction
4. If ERC-20 token: checks existing allowance; sends `approve` tx if needed
5. Submits bridge/swap tx via `onchainos wallet contract-call` to LiFiDiamond
6. Returns txHash and LI.FI explorer link

**Examples:**
```
# Bridge 5 USDC from Base to Arbitrum (dry-run preview)
lifi --chain 8453 swap --to-chain 42161 --from-token USDC --to-token USDC --amount 5000000 --dry-run

# Execute the bridge (will ask user to confirm)
lifi --chain 8453 swap --to-chain 42161 --from-token USDC --to-token USDC --amount 5000000
```

**Note:** After bridging, track status with:
```
lifi get-status --tx-hash <TX_HASH> --from-chain 8453 --to-chain 42161
```

---

## Chain IDs Reference

| Chain | ID |
|-------|----|
| Ethereum | 1 |
| Base | 8453 |
| Arbitrum | 42161 |
| Polygon | 137 |
| Optimism | 10 |
| BSC | 56 |
| Avalanche | 43114 |
| zkSync Era | 324 |
| Linea | 59144 |

---

## Security Notes

- All bridge/swap transactions are confirmed by the user before execution
- The LiFiDiamond contract is audited and used by millions of users
- Always verify the destination address and amounts before confirming
- Cross-chain transfers are irreversible once broadcast
