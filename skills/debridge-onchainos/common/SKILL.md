---
name: debridge-common
description: >
  Shared prerequisite for all deBridge agent skills. Runs three stages:
  ENVIRONMENT_DETECTION (CLI, MCP Desktop, browser, headless, chat-only),
  ACCESS_SETUP (streaming MCP, stdio MCP via @debridge-finance/debridge-mcp, SDK), and WALLET_DISCOVERY
  (auto-discover all signers and resolve wallet addresses). Run this first
  before any deBridge operation. Use whenever the user mentions deBridge for
  the first time in a session, asks about supported chains, needs to connect
  to deBridge MCP, or wants to check what signing methods are available. Also
  use when troubleshooting deBridge connectivity, checking environment
  capabilities, or setting up RPC endpoints.
license: MIT
metadata:
  author: deBridge
  version: "0.1.0"
---

# Environment Discovery

## Quick Reference

| Want to...                    | Go to                                    |
|-------------------------------|------------------------------------------|
| Detect environment type       | ENVIRONMENT_DETECTION below               |
| Refresh skills to latest      | Skill Freshness Check below              |
| Connect to deBridge MCP       | ACCESS_SETUP below + [mcp-setup.md](mcp-setup.md) |
| Discover wallets and signers  | WALLET_DISCOVERY below                   |
| Look up chain IDs and tokens  | [chain-config.md](chain-config.md)       |
| Discover RPC endpoints        | [rpc-discovery.md](rpc-discovery.md)     |
| Run bundled helper scripts    | `scripts/` directory (balance, allowance, convert, RPC) |
| Connect to MCP (all methods)  | [mcpc-usage.md](mcpc-usage.md)           |
| Swap or bridge tokens         | ../swap/SKILL.md                         |
| Set up a wallet               | ../wallets/SKILL.md                      |

## Detection Output

After completing all three phases, record:

```
Environment: <CLI | MCP Desktop | Browser | Headless | Chat-only>
Access:      <streaming-mcp | stdio-mcp | manual>
Signer:      <ows | env-privkey | foundry-cast | browser-wallet | ethers-viem | mcp-wallet | none>
Wallets:
  <signer_name> "<wallet_label>":
    EVM:    <0x_address>
    Solana: <base58_address>
```

Downstream skills use these values to select the right code paths. The `Wallets` section contains resolved standard addresses discovered during WALLET_DISCOVERY — pass these addresses (never wallet names) to all scripts and tools.

---

## ENVIRONMENT_DETECTION

Run checks in order. Stop at the first match.

### 1.1 CLI Agent

The agent can execute shell commands and has a runtime available.

Detection:
```bash
which node && echo "node available"
```

If bash works AND `node` is found → **Environment = CLI**.

Capabilities: full filesystem, package install (`npm`), can run MCP stdio server locally, can read environment variables.

### 1.2 MCP Desktop

The agent has MCP tools but limited or no bash access.

Detection: tool list includes any `mcp__debridge__*` tool. The agent is running inside Claude Desktop, Cursor, Windsurf, or an IDE with MCP support.

If MCP tools visible AND bash is unavailable or restricted → **Environment = MCP Desktop**.

Capabilities: MCP tool calls, may have file read/write via IDE, cannot install packages.

### 1.3 Browser

The agent runs in a browser context.

Detection: `window.ethereum` or EIP-1193 provider is accessible.

If browser APIs available → **Environment = Browser**.

Capabilities: injected wallet, DOM access, HTTP fetch. Cannot run local commands.

### 1.4 Headless / Autonomous

The agent runs programmatically without direct user interaction.

Detection: running inside OpenHands, CrewAI, LangChain, AutoGPT, or a custom SDK application. Has network access. May or may not have bash.

If programmatic agent framework detected → **Environment = Headless**.

Capabilities: varies by framework. Check tool list and bash availability individually.

### 1.5 Chat-Only (Fallback)

None of the above matched. The agent has no tool access.

**Environment = Chat-only**. All instructions become guidance for the user to execute manually.

---

## Skill Freshness Check

Optional: if skills may be outdated, read [skill-freshness.md](skill-freshness.md) for update methods (GitHub fetch, MCP resources, llms.txt). Otherwise proceed with bundled skills.

---

## Installing npm Packages

When **Environment = CLI** or **Headless** with Node.js available, npm packages (MCP servers, SDKs, CLIs, utilities) can be installed in two ways:

**`npx -y <pkg>`** — downloads, runs once, discards. Use for:
- First-time exploration or trying a tool
- One-off queries during a conversation
- CI/CD pipelines and ephemeral environments
- Any situation where the package is not needed again

**`npm install -g <pkg>`** — installs permanently. Use for:
- Agent harnesses that start the package repeatedly
- Long-lived processes and recurring scripts
- Projects that need reproducible, version-pinned dependencies (add to `devDependencies` in `package.json` instead of `-g`)

| Scenario | Command | Why |
|----------|---------|-----|
| Connect to deBridge MCP | `claude mcp add --transport http debridge https://agents.debridge.com/mcp` | Streaming, no install needed |
| Try a CLI tool | `npx -y ethers` | Quick one-shot use |
| Build a trading bot | `npm install ethers viem` | Pinned in `package.json`, no re-download |
| CI/CD pipeline | `npx -y <pkg>` | Clean environment each run |

This applies to all npm packages referenced in downstream skills — MCP servers, signing libraries, SDKs, and utilities.

### Connecting to deBridge MCP Without Native Streamable HTTP Support

For environments that support Streamable HTTP, connect directly to `https://agents.debridge.com/mcp` (see [mcp-setup.md](mcp-setup.md)). For environments that only support stdio transport, use `@debridge-finance/debridge-mcp` — a thin stdio proxy. Read [mcpc-usage.md](mcpc-usage.md) for all connection methods.

Quick start (stdio): `claude mcp add debridge npx -- -y @debridge-finance/debridge-mcp@latest`

---

## ACCESS_SETUP

### 2.1 Probe for Existing MCP Connection

Call `mcp__debridge__get_supported_chains` (no parameters).

- **Returns chain data** → MCP is already connected. Access = **streaming-mcp** or **stdio-mcp**. Skip to WALLET_DISCOVERY.
- **Tool not found** → MCP not connected. Continue to 2.2.

### 2.2 Set Up MCP by Environment

| Environment  | Recommended Method | Action                                             |
|--------------|--------------------|----------------------------------------------------|
| CLI          | streaming-mcp      | `claude mcp add --transport http debridge https://agents.debridge.com/mcp` |
| CLI (stdio)  | stdio proxy        | `claude mcp add debridge npx -- -y @debridge-finance/debridge-mcp@latest` |
| MCP Desktop  | streaming-mcp      | Read [mcp-setup.md](mcp-setup.md) for client config |
| Browser      | manual             | Guide user to set up an MCP-capable environment    |
| Headless     | stdio-mcp          | Read [mcp-setup.md](mcp-setup.md) for SDK or stdio proxy setup |
| Chat-only    | manual             | Guide user to set up an MCP-capable environment    |

#### CLI: Streamable HTTP (preferred)

If the environment supports Streamable HTTP transport, connect directly to the hosted endpoint:

```bash
claude mcp add --transport http debridge https://agents.debridge.com/mcp
```

This requires restarting the Claude Code session. After restart, all `mcp__debridge__*` tools are available.

#### CLI: Stdio Proxy (fallback for stdio-only environments)

If the environment only supports stdio transport, use `@debridge-finance/debridge-mcp` as a local proxy:

```bash
claude mcp add debridge npx -- -y @debridge-finance/debridge-mcp@latest
```

This requires restarting the Claude Code session. The proxy forwards all requests to `https://agents.debridge.com/mcp` transparently.

Read [mcpc-usage.md](mcpc-usage.md) for all connection methods and configuration details. For Claude Desktop, Cursor, or programmatic SDK setup, read [mcp-setup.md](mcp-setup.md).

### 2.3 Future Access Methods

These are **not available yet** but will be supported:

- **`@debridge/sdk`** — TypeScript/JavaScript SDK, installable via npm. COMING SOON.
- **`@debridge/cli`** — Command-line tool for bridge/swap. COMING SOON.

When available, this skill will add detection and routing for them.

### 2.4 Verify Connection

After setup, call `mcp__debridge__get_supported_chains` again.
- Returns chain data → MCP is working. Proceed to WALLET_DISCOVERY.
- Fails → read [mcp-setup.md](mcp-setup.md) troubleshooting section.

---

## WALLET_DISCOVERY

A signer is needed for on-chain transactions (bridge, swap, token approval).
deBridge requires signing EIP-712 typed data messages and standard EVM transactions.

**Auto-discovery is mandatory.** When the user asks to check balances, bridge, swap, or perform any on-chain operation, the agent MUST automatically discover all available signers and resolve their wallet addresses — never ask the user for an address or wallet name. Run the checks below in order, collect ALL matches (do not stop at the first), then use the highest-priority signer for signing operations.

After discovery, record all found wallets in the Detection Output (see top of this file) so downstream skills can use them without re-running discovery.

Check in order. Collect all matches.

### 3.1 OWS (Open Wallet Standard)

**Detection:**
```bash
which ows && echo "available"
```

Or check for the Node.js SDK:
```bash
node -e "require('@open-wallet-standard/core')" 2>/dev/null && echo "ows-node"
```

If any available → **Signer = ows**.

**Address discovery — resolve all addresses now:**
```bash
ows wallet list
```

Parse the output to extract addresses for each chain. The output format is:
```
ID:      <uuid>
Name:    <wallet_name>
Secured: ✓ (encrypted)
  eip155:1 → 0x<evm_address>
  solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp → <base58_solana_address>
  ...
Created: <timestamp>
```

Parsing rules:
- Wallet name is on the `Name:` line (e.g., `Name:    default`).
- Each chain address is indented and follows the pattern `<chain_namespace>:<chain_ref> → <address>`.
- EVM address: extract from any `eip155:` line — all EVM chains share the same address.
- Solana address: extract from the `solana:` line.
- If multiple wallets exist, discover addresses for ALL of them.

**Store the resolved standard addresses** (e.g., `0x000A...9c30` for EVM, `B7Z1...SUr` for Solana). Always pass these standard addresses — never wallet names — to scripts and downstream tools.

OWS provides local self-custody signing with encrypted keys, policy-gated access, and multi-chain support (EVM, Solana, Tron). Recommended for CLI and agent environments — see ../wallets/SKILL.md Option 1 for setup.

### 3.2 Private Key in Environment

Use the bundled `env-keys.mjs` script to scan environment variables and `.env` files in one step. The script never prints raw key values — only variable names, sources, chain types, and derived addresses.

**Detection and address discovery (single command):**
```bash
node scripts/env-keys.mjs
```

Human-readable output shows a table of discovered keys with their source, chain, and derived address. Use `--json` for machine-readable output:
```bash
node scripts/env-keys.mjs --json
```

JSON output format:
```json
[
  { "name": "PRIVATE_KEY", "source": "env", "chain": "evm", "address": "0x...", "format": "hex-0x" },
  { "name": "SOLANA_PRIVATE_KEY", "source": ".env", "chain": "solana", "address": "B7Z1...", "format": "json-array" }
]
```

If any keys found → **Signer = env-privkey**. Record the derived addresses in the Detection Output.

The script scans (in priority order):
1. Environment variables: `PRIVATE_KEY`, `*_PRIVATE_KEY`, `*_KEY` (filtered by format)
2. `.env` in current directory
3. `.env.local` in current directory
4. `~/.env` in home directory

**Security warnings (issued automatically by the script):**
- Keys found in `.env` files on disk trigger a plaintext storage warning
- Keys found in environment variables trigger a weaker warning
- Both recommend migrating to OWS for encrypted self-custody

🚨 **If keys are found in files on disk**, relay the script's warning and recommend:
> 1. Move to OWS: `curl -fsSL https://docs.openwallet.sh/install.sh | bash && ows wallet create`
> 2. Delete the file containing the key after migrating
> 3. Rotate the key if the file was ever committed to git or shared

⚠️ CAUTION: Never log, print, or include private key values in any output. The script enforces this — do not bypass it with ad-hoc shell commands like `echo $PRIVATE_KEY` or `grep` that could leak key material.

### 3.3 Foundry Cast

**Detection:**
```bash
which cast && echo "available"
```

If available → **Signer = foundry-cast**.

Cast supports EIP-712 signing (`cast wallet sign --data`) and raw transaction sending (`cast send`). Requires a keystore or `--private-key` flag.

**Address discovery:**
```bash
# List cast wallets/keystores
cast wallet list 2>/dev/null
```

### 3.4 Browser Wallet (EIP-1193)

If `window.ethereum` exists → **Signer = browser-wallet**.

Supports `eth_signTypedData_v4` for EIP-712 and `eth_sendTransaction` for raw transactions.

**Address discovery:** Call `eth_requestAccounts` to get the connected address.

### 3.5 ethers.js or viem

**Detection:**
```bash
node -e "require('ethers')" 2>/dev/null && echo "ethers"
node -e "require('viem')" 2>/dev/null && echo "viem"
```

If either available → **Signer = ethers-viem**.

Both support EIP-712 via `signer.signTypedData()` (ethers) or `walletClient.signTypedData()` (viem). Both can send raw transactions.

**Address discovery:** Requires a private key or keystore — address comes from the key discovery in 3.2.

### 3.6 MCP-Managed Wallet

Check if MCP tools include a signing or wallet tool:
- `mcp__privy__eth_sendTransaction` → Privy embedded wallet is available.
- Any other MCP signing tool → compatible MCP wallet.

If available → **Signer = mcp-wallet**.

Privy MCP handles signing server-side (keys in TEE). The agent passes `create_tx` output directly to Privy's `eth_sendTransaction` — no local key or RPC needed. See ../wallets/privy-embedded.md for setup.

**Address discovery:** Call the MCP wallet's address/list endpoint to get managed addresses.

### 3.8 No Signer Available

If none matched → **Signer = none**.

Guide the user to set up a signer:
- Recommended: install OWS (`curl -fsSL https://docs.openwallet.sh/install.sh | bash`) — multi-chain, encrypted keys, works with CLI/Node.js/Python
- Quick start: set `PRIVATE_KEY` environment variable (EVM-only, plaintext)
- For development: install Foundry (`curl -L https://foundry.paradigm.xyz | bash && foundryup`)
- For delegated custody: set up Privy embedded wallet (see ../wallets/privy-embedded.md)
- For all options: read ../wallets/SKILL.md

### 3.9 Discovery Output

After completing all checks, record discovered wallets. Example:

```
Signers found: ows, env-privkey
Primary signer: ows

Wallets:
  OWS "default":
    EVM:    0x000A5539cD9505b44575c56f929C657c73899c30
    Solana: B7Z1whe4TX3tVXwb93Nsd9U4f4QZfnuzm5DyUnKxVSUr
  env-privkey:
    EVM:    0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18
```

Pass these resolved addresses to all downstream operations — balance scripts, signing, bridging. Never pass wallet names to scripts.

---

## MCP Tool Reference

| MCP Tool                                        | Purpose                                    |
|-------------------------------------------------|--------------------------------------------|
| `mcp__debridge__get_instructions`               | Return the server's canonical workflow guide — call first |
| `mcp__debridge__get_supported_chains`           | List supported chains with IDs and names   |
| `mcp__debridge__search_tokens`                  | Find token by name, symbol, or address     |
| `mcp__debridge__create_tx`                      | Build cross-chain bridge/swap transaction  |
| `mcp__debridge__transaction_same_chain_swap`    | Build same-chain swap transaction          |

All MCP tools expect token amounts in **raw units** (the smallest indivisible unit: wei for EVM, lamports for Solana) passed as strings. See [chain-config.md](chain-config.md) for decimals and conversion.

---

## Common Errors

| Error                      | Cause                    | Fix                                                  |
|----------------------------|--------------------------|------------------------------------------------------|
| MCP tool not found         | Server not connected     | Re-run ACCESS_SETUP                                  |
| `npx` not found            | Node.js not installed    | Install Node.js 18+                                  |
| Permission denied on key   | Env var not exported      | `export PRIVATE_KEY=...` in shell config             |
| Chain ID not recognized    | Wrong ID format          | Use deBridge chain IDs from [chain-config.md](chain-config.md) |
| Amount format error        | Human-readable passed    | Convert to raw units first                           |

## References

- [chain-config.md](chain-config.md) — Chain IDs, tokens, decimals, amount conversion
- [mcp-setup.md](mcp-setup.md) — MCP configuration for all environments
- [rpc-discovery.md](rpc-discovery.md) — RPC endpoint discovery via Chainlist
