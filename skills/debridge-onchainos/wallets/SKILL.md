---
name: debridge-wallets
description: >
  Set up a wallet for deBridge transactions. Use when the user has no signer
  available, needs to create a new wallet, or wants to configure wallet
  access for an AI agent. Covers OWS local self-custody wallets (recommended
  — multi-chain, encrypted keys, CLI/JS), generating a raw private key,
  creating a Foundry keystore, installing a browser wallet (MetaMask),
  and setting up Privy embedded wallets for zero-UI autonomous agent trading.
  Use this skill when: the user says "I don't have a wallet", "how do I set
  up a wallet", "create a new wallet", "generate an address", "I need a
  wallet for bridging", "set up OWS", "OWS wallet", "set up Privy",
  "embedded wallet for my agent", "keystore setup", or when WALLET_DISCOVERY
  detected no signer. Also relevant for "conversational trading setup" and
  "autonomous agent wallet".
license: MIT
metadata:
  author: deBridge
  version: "0.1.0"
---

# Wallet Setup

PREREQUISITE: Read ../common/SKILL.md for environment detection, auth, and chain configuration.

Use this skill when WALLET_DISCOVERY detected **Signer = none**. Choose the method that matches your environment.

## Quick Reference

| Environment       | Recommended method       | Go to                                        |
|-------------------|--------------------------|----------------------------------------------|
| CLI / Agent       | **OWS** (recommended)   | Option 1 below                               |
| CLI + Node.js     | Raw private key + env var| Option 2 below                               |
| CLI + Foundry     | Foundry keystore         | Option 3 below                               |
| Browser           | Install MetaMask         | Option 4 below                               |
| Agent (zero-UI)   | Privy embedded wallet    | Option 5 / [privy-embedded.md](privy-embedded.md) |

**Why OWS first?** It supports all deBridge chains (EVM, Solana, Tron) from a single wallet, encrypts keys at rest with policy-gated signing, works across CLI/Node.js environments, and generates addresses for all chains in one step. More secure than raw private keys, simpler than Foundry for multi-chain use.

After setup, re-run WALLET_DISCOVERY to confirm the signer is detected, then proceed to ../signing/SKILL.md.

---

## Option 1: OWS Wallet (Open Wallet Standard) — Recommended

Local self-custody wallet — private keys encrypted at rest on the user's machine, decrypted only in-process during signing, then wiped from memory. Policy-gated access, multi-chain support (EVM, Solana, Tron, Bitcoin, Cosmos, TON, Sui, and more).

### Install

Pick the method that matches your environment:

| Environment | Command | What it installs |
|-------------|---------|------------------|
| Any (full suite) | `curl -fsSL https://docs.openwallet.sh/install.sh \| bash` | CLI + Node.js SDK |
| Node.js only | `npm install @open-wallet-standard/core` | Node.js SDK (prebuilt binaries, no Rust needed) |
| From source | `git clone https://github.com/open-wallet-standard/core.git && cd core/ows && cargo build --workspace --release` | Rust build |

The full suite (`curl`) is recommended for agents — it gives you the CLI plus the Node.js SDK.

### Create Wallet

```bash
ows wallet create
```

This generates keys for all supported chains in one step. Record the addresses from the output.

### Verify

```bash
ows wallet list
```

### Fund the Wallet

Send tokens to the OWS wallet address on the source chain before bridging.

### Signing

Proceed to ../signing/SKILL.md — the signing skill routes to [ows-signing.md](../signing/ows-signing.md) for OWS-specific signing flows (EVM direct, Solana pipeline, Tron).

---

## Option 2: Private Key via Environment Variable

Fastest path if you only need a single EVM chain. Generates a random private key and stores it in the shell environment. Less secure than OWS — the key is stored in plaintext.

### Generate with Node.js

```bash
node -e "const w = require('ethers').Wallet.createRandom(); console.log('Address:', w.address); console.log('Private key:', w.privateKey)"
```

If ethers is not installed:

```bash
npx -y ethers node -e "const w = require('ethers').Wallet.createRandom(); console.log('Address:', w.address); console.log('Private key:', w.privateKey)"
```

### Generate with OpenSSL

```bash
openssl rand -hex 32
```

This outputs a raw 32-byte hex string. Prefix with `0x` for use as a private key.

### Store in Environment

Add to shell profile (`~/.bashrc`, `~/.zshrc`, or `.env`):

```bash
export PRIVATE_KEY="0x<generated_key>"
```

Then reload: `source ~/.bashrc`

⚠️ CAUTION: Never commit private keys to git. Add `.env` to `.gitignore`.

### Derive Address

```bash
# ethers
node -e "const w = new (require('ethers').Wallet)('$PRIVATE_KEY'); console.log(w.address)"

# cast
cast wallet address --private-key "$PRIVATE_KEY"
```

### Fund the Wallet

The new wallet has zero balance. Send native tokens (ETH, etc.) to the derived address before bridging. Use a faucet for testnet work.

---

## Option 3: Foundry Keystore

More secure than a raw environment variable — the private key is encrypted at rest. EVM-only.

### Prerequisites

```bash
which cast || (curl -L https://foundry.paradigm.xyz | bash && foundryup)
```

### Create Keystore

```bash
cast wallet new ~/.foundry/keystores/debridge
```

This generates a new key pair and encrypts it with a password. Record the address from the output.

### Or Import Existing Key

```bash
cast wallet import debridge --interactive
```

Enter the private key and a password when prompted.

### Use in Commands

```bash
cast send "$TO" "$DATA" --account debridge --rpc-url "$RPC_URL"
```

Cast will prompt for the keystore password.

### Fund the Wallet

Send native tokens to the keystore address before bridging.

---

## Option 4: Browser Wallet (MetaMask)

For browser-based environments.

### Install

1. Go to [metamask.io/download](https://metamask.io/download).
2. Install the browser extension.
3. Create a new wallet or import an existing one.
4. Record the wallet address.

### Connect to deBridge Chains

MetaMask ships with Ethereum mainnet. Add other chains:

1. Open MetaMask → Settings → Networks → Add Network.
2. Use [chainlist.org](https://chainlist.org) to auto-add chains by name.
3. Or add manually using chain IDs from ../common/chain-config.md.

### Fund the Wallet

Send native tokens to the MetaMask address on the source chain before bridging.

---

## Option 5: Privy Embedded Wallet

Server-side wallets managed by Privy infrastructure (keys secured in TEEs). The agent signs and broadcasts transactions via Privy MCP — no browser, no wallet popup, no local keys. Best for autonomous agent workflows that need delegated custody.

Read [privy-embedded.md](privy-embedded.md) for full setup.

Quick summary:
1. Create a Privy account at [dashboard.privy.io](https://dashboard.privy.io) and get App ID + App Secret.
2. Install Privy MCP server and add it alongside deBridge MCP.
3. Create wallets via Privy MCP (`create_wallet` for EVM and/or Solana).
4. Fund the wallet on the source chain.
5. The agent passes `create_tx` output directly to Privy's `eth_sendTransaction` — no format conversion needed.

---

## After Setup

For Option 1 (OWS):
1. Verify OWS CLI is available (`ows wallet list`).
2. Proceed to ../signing/SKILL.md — it routes to the OWS signing reference.
3. Then to ../swap/SKILL.md for the operation.

For Options 2–4:
1. Re-run WALLET_DISCOVERY to verify the signer is detected.
2. Proceed to ../signing/SKILL.md for transaction signing.
3. Then to ../swap/SKILL.md for the operation.

For Option 5 (Privy):
1. Verify both deBridge and Privy MCPs are connected.
2. The agent uses deBridge MCP for routing and Privy MCP for signing — no separate signing step needed.
3. Proceed directly to ../swap/SKILL.md.

## References

- [privy-embedded.md](privy-embedded.md) — Full Privy embedded wallet setup and integration
