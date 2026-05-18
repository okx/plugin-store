---
name: debridge-analytics
description: >
  Query DeFi analytics and on-chain data from third-party MCP servers to
  make informed bridging and swapping decisions. Covers token prices and
  exchange data (CoinGecko, Crypto.com, mcp-crypto-price), on-chain
  lookups and multi-chain analytics (Blockscout, Hive Intelligence),
  protocol TVL, yields, and vault risk (DefiLlama, Philidor), and DEX
  pool liquidity (DexPaprika). All servers are free and require no API
  key. Use this skill whenever the user asks about token prices, wallet
  balances, portfolio overview, transaction history, protocol TVL, DEX
  liquidity, yield opportunities, orderbook depth, vault risk, or
  on-chain data. Also use for: "what's the price of ETH", "check my
  balance", "show me the TVL of Aave", "is there enough liquidity to
  swap", "look up this transaction", "what's the risk of this vault".
license: MIT
metadata:
  author: deBridge
  version: "0.1.0"
---

# DeFi Analytics

PREREQUISITE: Read ../common/SKILL.md for environment detection, auth, and chain configuration.

Third-party MCP servers provide analytics data useful before, during, and after deBridge operations: token prices, wallet balances, protocol TVL, DEX liquidity, and on-chain transaction details. All analytics MCPs listed here are free and require no API key.

## Quick Reference

| Want to...                              | MCP                          | Go to                                              |
|-----------------------------------------|------------------------------|----------------------------------------------------|
| Get token prices and market data        | CoinGecko, Crypto.com        | [token-prices.md](token-prices.md)                 |
| Look up address, tx, or contract        | Blockscout, Hive             | [onchain-explorer.md](onchain-explorer.md)         |
| Check protocol TVL, fees, yields        | DefiLlama, Philidor          | [defi-tvl.md](defi-tvl.md)                         |
| Analyze DEX pools, OHLCV, trades        | DexPaprika                   | [dex-pools.md](dex-pools.md)                       |
| Assess DeFi vault risk                  | Philidor                     | [defi-tvl.md](defi-tvl.md)                         |
| Cross-chain swap quotes and volume      | SODAX                        | See sodax endpoint below                           |
| Query balances directly (no MCP)        | ethers, viem, cast, web3     | Direct Balance Queries below                       |

---

## Installing Analytics MCPs

### npx vs npm install

Use **`npx`** for one-shot exploration — the package is fetched, executed once, and not retained:

```bash
npx -y @coingecko/coingecko-mcp         # try CoinGecko MCP
npx -y @nic0xflamel/defillama-mcp-server # try DefiLlama MCP
npx dexpaprika-mcp                       # try DexPaprika MCP
npx -y mcp-crypto-price                  # try mcp-crypto-price MCP
```

Use **`npm install -g`** (or add to `devDependencies`) when building a persistent agent harness, a recurring script, or a long-lived process:

```bash
npm install -g @coingecko/coingecko-mcp  # always available, faster startup
```

| Scenario                                    | Use         |
|---------------------------------------------|-------------|
| First time trying an MCP                    | `npx -y`    |
| One-off query during a conversation         | `npx -y`    |
| CI/CD pipeline, ephemeral environment       | `npx -y`    |
| Agent harness running the MCP repeatedly    | `npm install -g` or `devDependencies` |
| Project with pinned MCP versions            | `devDependencies` in `package.json`   |

### Adding to Claude Desktop

All analytics MCPs use the same config pattern. Add to the Claude Desktop config file (see ../common/mcp-setup.md for file location):

For hosted MCPs (CoinGecko, Blockscout) that expose a remote endpoint:

```json
{
  "mcpServers": {
    "<mcp-name>": {
      "type": "streamable-http",
      "url": "<hosted-endpoint>"
    }
  }
}
```

### Adding to Claude Code (CLI)

Stdio (local):
```bash
claude mcp add <name> -- npx -y <package-name>
```

Streaming (hosted):
```bash
claude mcp add --transport http <name> <hosted-url>
```

---

## API Key Requirements

All analytics MCPs are free and require no API key:

| MCP              | API Key Required | Endpoint                                          |
|------------------|------------------|---------------------------------------------------|
| CoinGecko        | No               | Hosted: `https://mcp.api.coingecko.com/mcp`      |
| Crypto.com       | No               | Hosted: `https://mcp.crypto.com/market-data/mcp`  |
| Blockscout       | No               | Hosted: `https://mcp.blockscout.com/mcp`          |
| Hive Intelligence| No               | Hosted: `https://hiveintelligence.xyz/mcp`        |
| Philidor         | No               | Hosted: `https://mcp.philidor.io/api/mcp`        |
| DexPaprika       | No               | Hosted: `https://mcp.dexpaprika.com/streamable-http` |
| DefiLlama        | No               | Local: `npx -y @nic0xflamel/defillama-mcp-server` |
| mcp-crypto-price | No               | Local: `npx -y mcp-crypto-price`                  |
| SODAX            | No               | Hosted: `https://builders.sodax.com/mcp`          |
| Arcadia Finance  | No               | Hosted: `https://mcp.arcadia.finance/mcp`         |

---

## Use Case Routing

### Before a Bridge or Swap

1. **Check token price** → [token-prices.md](token-prices.md) — verify the token is priced as expected before committing.
2. **Check destination pool liquidity** → [dex-pools.md](dex-pools.md) — ensure the destination chain has sufficient liquidity.
3. **Check wallet balances** → Direct Balance Queries below, or Blockscout `get_tokens_by_address` per chain.

### After a Bridge or Swap

4. **Verify transaction on-chain** → [onchain-explorer.md](onchain-explorer.md) — confirm the tx landed on the destination chain.
5. **Check received balance** → Blockscout `get_tokens_by_address` or Direct Balance Queries below.

### Research and Analysis

6. **Protocol TVL and yields** → [defi-tvl.md](defi-tvl.md) — compare protocols, check yield opportunities on destination chains.

---

## Direct Balance Queries (No MCP Required)

When no analytics MCP is connected, query balances directly using the signer/runtime detected during WALLET_DISCOVERY in ../common/SKILL.md. Route based on available tool:

| Chain / Runtime       | Read this file                             |
|-----------------------|--------------------------------------------|
| EVM (ethers / viem)   | [balance-ethers.md](balance-ethers.md)     |
| EVM (Foundry cast)    | [balance-cast.md](balance-cast.md)         |
| Solana                | [balance-solana.md](balance-solana.md)     |
| none                  | Install one: `npm install ethers` is fastest |

### Auto-Discovery Balance Flow

⚠️ **STOP — Do NOT ask the user for an address or chain.** When the user says "check my balance", "what's my balance", "show balances", or anything balance-related, skip all clarifying questions and immediately run discovery. This is the #1 anti-pattern: asking "which chain?" or "what address?" when the skill requires auto-discovery.

The agent MUST:

1. **Discover wallets FIRST** — run ALL applicable discovery methods and collect addresses:
   - **OWS**: `ows wallet list` → parse EVM (`eip155:` lines) and Solana (`solana:` lines) addresses.
   - **env-keys**: `node ../common/scripts/env-keys.mjs --json` → collect addresses from environment variables and `.env` files. This replaces ad-hoc shell commands like `test -n "$PRIVATE_KEY"` — always use the script.
   - **Cast**: `cast wallet list` if Foundry is available.
   - Do NOT ask the user for an address — discover it.
2. **Query native balances** across ALL chains (both EVM and Solana if addresses exist for both). "All" is the default — never ask which chain.
   - EVM: `node scripts/balance-evm.mjs <address> --all`
   - Solana: `node scripts/balance-solana.mjs <address> --tokens`
3. **Query ERC-20 token balances** — native balance scripts only report native tokens. For ERC-20/SPL discovery the agent MUST also use analytics MCPs. Try each in order until one succeeds:
   - **Blockscout MCP** (preferred): call `mcp__blockscout__get_tokens_by_address` with `address` and `chain_id` (as string) for each major EVM chain (at minimum: `"1"`, `"56"`, `"137"`, `"42161"`, `"10"`, `"8453"`). Hosted at `https://mcp.blockscout.com/mcp` — no API key.
   - **Hive Intelligence** (fallback): call `mcp__hive_portfolio__get_wallet_token_balances` with `id` = address and `is_all` = true. Hosted at `https://mcp.hiveintelligence.xyz/hive_portfolio_wallet/mcp` — no API key.
   - **ethers.js `balanceOf`** (last resort): only if you already know specific token contract addresses from context. Cannot enumerate unknown tokens — skip if no addresses known.
   - Do NOT report only native balances and stop. ERC-20 discovery is required when analytics MCPs are available.
4. **Report results** for every wallet, every chain, both native and token balances. Group by chain for readability.

### Bundled Scripts

**Scripts accept only standard addresses — never wallet names.**

```bash
# EVM — pass a 0x address, query all deBridge EVM chains
node scripts/balance-evm.mjs 0x000A5539cD9505b44575c56f929C657c73899c30 --all

# EVM — pass a 0x address, query specific chains only
node scripts/balance-evm.mjs 0x000A5539cD9505b44575c56f929C657c73899c30 --chains 1,137,42161

# EVM — query a specific ERC-20 token balance (e.g., USDC on Polygon)
node scripts/balance-evm.mjs 0x000A5539cD9505b44575c56f929C657c73899c30 --chains 137 --token 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359

# Solana — pass a base58 address, query native SOL + SPL tokens
node scripts/balance-solana.mjs B7Z1whe4TX3tVXwb93Nsd9U4f4QZfnuzm5DyUnKxVSUr --tokens

# JSON output for piping
node scripts/balance-evm.mjs 0x000A5539cD9505b44575c56f929C657c73899c30 --all --json
node scripts/balance-solana.mjs B7Z1whe4TX3tVXwb93Nsd9U4f4QZfnuzm5DyUnKxVSUr --tokens --json
```

**Arguments:**
- First argument: a standard blockchain address (EVM `0x` hex, 42 chars; or Solana base58, 32-44 chars). **Required.**
- `--all`: (EVM only) query all deBridge-supported EVM chains.
- `--chains <ids>`: (EVM only) comma-separated chain IDs.
- `--token <addr>`: (EVM only) query a specific ERC-20 token balance instead of native balance. Reads decimals and symbol on-chain.
- `--tokens`: (Solana only) also list non-zero SPL token balances.
- `--json`: output as JSON.

**Do NOT pass OWS wallet names, ENS names, or any non-address string.** Resolve addresses first via WALLET_DISCOVERY, then pass the resolved address to the script.

All direct query methods require an RPC endpoint. Resolve RPCs in this order:
1. Environment variable (`$RPC_URL`, `$ETH_RPC_URL`).
2. User-provided URL.
3. Discover from Chainlist — read ../common/rpc-discovery.md.

### When to Use Direct Queries vs MCP

| Scenario | Use |
|----------|-----|
| Native balances, multi-chain | Bundled script (`balance-evm.mjs --all`) — parallel RPC calls |
| ERC-20 token discovery | Blockscout MCP `get_tokens_by_address` — enumerates all tokens without knowing addresses |
| Full portfolio ("check my balance") | **Both**: bundled script for native + Blockscout/Hive MCP for ERC-20 tokens |
| No MCP available at all | Bundled script for native only — warn user that ERC-20 tokens are not shown |

---

## When No Analytics MCP Is Available

If no analytics MCPs are installed, you can still gather basic data:

| Need | Fallback |
|------|----------|
| Token balance | Direct RPC query — see Direct Balance Queries above |
| Token price | deBridge MCP itself: compare `create_tx` input/output amounts for an implied exchange rate |
| Transaction verification | Use an RPC `eth_getTransactionReceipt` call or the bundled scripts |
| Pool liquidity / TVL / yields | No direct fallback — suggest installing CoinGecko MCP (free, no key): `claude mcp add --transport http coingecko https://mcp.api.coingecko.com/mcp` |

For a quick start with zero setup, CoinGecko (hosted, no API key), Crypto.com (hosted, no API key), and Blockscout (hosted, no API key) cover most pre-swap research needs.

## Common Errors

| Error                          | Cause                         | Fix                                                      |
|--------------------------------|-------------------------------|----------------------------------------------------------|
| MCP tool not found             | MCP not installed/configured  | Follow installation in the relevant reference file        |
| Rate limited (429)             | Too many requests             | Add delay between calls or switch to a different free MCP |
| `npx` hangs on first run      | Large package download        | Use `npm install -g` for persistent use                   |
| Chain not supported            | MCP doesn't cover that chain  | Check chain support in each reference file                |

