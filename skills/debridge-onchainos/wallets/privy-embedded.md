---
title: Privy Embedded Wallet Setup
impact: HIGH
impactDescription: "Server-side wallet for zero-UI agent trading via Privy MCP"
tags: privy, embedded-wallet, mcp, tee, signing, agent-trading
---

# Privy Embedded Wallet

Server-side wallets managed by Privy (keys secured in TEEs). The agent routes trades via deBridge MCP and signs/broadcasts via Privy MCP — no browser, no dApp, no wallet popup.

```
User prompt → deBridge MCP (route + tx data) → Privy MCP (sign + broadcast) → On-chain
```

Works for cross-chain bridges, same-chain swaps, and cross-chain swap combos.

## Prerequisites

- Node.js 18+
- deBridge MCP configured (see ../common/mcp-setup.md)

## Step 1: Create Privy Account

This is the only step that requires a browser.

1. Open [dashboard.privy.io](https://dashboard.privy.io) and sign up.
2. Click "Create new app" (e.g., "deBridge Agent").
3. Navigate to **Settings → Basics → API Keys**.
4. Copy two values:
   - **App ID** — public, safe to expose.
   - **App Secret** — private. Copy immediately — Privy does not store it. If lost, regenerate.

⚠️ CAUTION: Never commit App Secret to git or include in skill content.

## Step 2: Install Privy MCP

### From source (current)

```bash
git clone https://github.com/privy-io/privy-mcp-server.git
cd privy-mcp-server
npm install && npm run build
```

Add to Claude Code from inside the cloned directory:

```bash
claude mcp add privy node -- dist/index.js \
  --env PRIVY_APP_ID=<your-app-id> \
  --env PRIVY_APP_SECRET=<your-app-secret>
```

### From npm (when available)

```bash
claude mcp add privy npx -- @privy-io/mcp-server \
  --env PRIVY_APP_ID=<your-app-id> \
  --env PRIVY_APP_SECRET=<your-app-secret>
```

### For Claude Desktop

Add to Claude Desktop config (see ../common/mcp-setup.md for config file location):

```json
{
  "mcpServers": {
    "debridge": {
      "type": "streamable-http",
      "url": "https://agents.debridge.com/mcp"
    },
    "privy": {
      "command": "node",
      "args": ["<path-to-privy-mcp-server>/dist/index.js"],
      "env": {
        "PRIVY_APP_ID": "<your-app-id>",
        "PRIVY_APP_SECRET": "<your-app-secret>"
      }
    }
  }
}
```

### Verify

```bash
claude mcp list
```

Both `debridge` and `privy` should show connected status.

## Step 3: Create Wallets

Ask the agent to create wallets via Privy MCP:

```
Using the Privy MCP, create wallets for me on Ethereum and Solana. Show me the addresses.
```

Privy creates embedded server-side wallets (keys managed in TEEs, never exposed). Record the wallet addresses.

## Step 4: Fund the Wallet

Send tokens to the Privy wallet address on the source chain:
- The swap/bridge amount in the source token (e.g., USDC)
- Native token for gas (e.g., ETH on Arbitrum)

Use an L2 chain (Arbitrum, Base, BSC) to minimize gas costs for testing.

## How the Handoff Works

deBridge MCP's `create_tx` returns standard EVM transaction data:

```json
{
  "to": "0xeF4fB24aD0916217251F553c0596F8Edc630EB66",
  "value": "1000000000000000",
  "data": "0xb9303701..."
}
```

Privy MCP's `eth_sendTransaction` accepts the same format:

```json
{
  "to": "0xeF4fB24aD0916217251F553c0596F8Edc630EB66",
  "value": "1000000000000000",
  "data": "0xb9303701...",
  "chain_id": 42161
}
```

The agent passes `create_tx` output directly to Privy's `eth_sendTransaction`. No format conversion needed. The full flow:

1. `mcp__debridge__create_tx` → returns tx data + order ID
2. If approval needed → `mcp__privy__eth_sendTransaction` with approval tx, wait for confirmation
3. `mcp__privy__eth_sendTransaction` with bridge/swap tx
4. Track order via monitoring (see ../swap/monitoring.md)

## Example Prompts

Cross-chain:
- "Swap 50 USDC from Base to Solana"
- "Bridge 0.1 ETH from Ethereum to Arbitrum"

Same-chain:
- "Swap 0.05 ETH to USDC on Arbitrum"
- "Trade 100 USDC for USDT on Base"

Cross-chain + swap combo:
- "Swap my ETH on Ethereum into USDC on Solana"

Utility:
- "Check my Privy wallet balances across all chains"
- "Get a deBridge quote for 500 USDC from Polygon to Solana, don't execute yet"

## Security Notes

- Privy wallet keys are managed in TEEs (Trusted Execution Environments) and never exposed.
- App Secret must not be committed to version control.
- For testing, use small amounts on L2 chains.
- Privy supports wallet policies and transaction limits for production use.
- deBridge MCP is read-only by itself — it generates tx data but cannot sign or broadcast.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Privy MCP disconnected | Verify App ID and App Secret. Remove and re-add: `claude mcp remove privy` then re-add |
| Insufficient funds | Wallet needs both swap token AND native gas token on source chain |
| Transaction not confirmed | Cross-chain orders settle in seconds. Check status via deBridge MCP or ../swap/monitoring.md |
| High price impact | Use L2 chains (Arbitrum, Base) instead of Ethereum mainnet for small amounts |
