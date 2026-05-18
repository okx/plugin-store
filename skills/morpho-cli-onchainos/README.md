# morpho-cli-onchainos

Drive the Morpho lending protocol from the terminal — queries vaults/markets/positions and prepares unsigned Morpho transactions with built-in simulation across all supported chains, with on-chain signing routed through Onchain OS.

Supported chains: ethereum, base, arbitrum, optimism, polygon, unichain, worldchain, katana, hyperevm, monad, stable

## Install

```bash
npx skills add <your-org>/morpho-cli-onchainos
```

This skill requires Onchain OS. On first use, the LLM will automatically run the install command in `## Pre-flight Checks`:

```bash
npx skills add okx/onchainos-skills
```

> Once published to the OKX Skills Marketplace, end users can install with a single `npx skills add` command. Until then, install via `git clone` or `cp -r` into `~/.agents/skills/morpho-cli-onchainos/`.

## First-time authentication (email wallet creation)

All on-chain signing is handled by Onchain OS inside a local TEE. This DApp never touches private keys or persists login state.

### Option A — Email OTP (recommended for individual users)

Onchain OS guides you through login on first signing call:

```bash
onchainos wallet login user@example.com     # sends OTP
onchainos wallet verify <6-digit-code>      # completes login
onchainos wallet status                     # confirm loggedIn: true
```

### Option B — API Key (automation / backend / CI)

```bash
export OKX_API_KEY=<your API Key>
export OKX_SECRET_KEY=<your Secret Key>
export OKX_PASSPHRASE=<your Passphrase>
onchainos wallet login
```

> To obtain credentials, visit the OKX Developer Portal (link provided by the technical team).

## Usage example

Trigger tools with natural language in your Agent:

> User: Deposit 1000 USDC into the highest-APY USDC vault on Base.

The Agent follows this flow:

1. Calls `queryVaults({ chain: "base", assetSymbol: "USDC", sort: "apy_desc", limit: 1 })`
2. Calls `prepareDeposit({ chain: "base", vaultAddress: <top>, userAddress: <user>, amount: "1000" })`
3. Tool returns `pending_sign` + `next_action.tool = 'onchainos wallet contract-call'`
4. Agent routes the approve through Onchain OS
5. Agent re-invokes `prepareDeposit` with the same params; tool returns the deposit tx as `pending_sign`
6. Agent routes the deposit through Onchain OS; tx confirms; done

## Security

- This DApp never reads, stores, or transmits the user's private key, seed phrase, or keystore
- All `pending_sign` transactions are signed exclusively through Onchain OS
- `ethers.Wallet`, `signTransaction`, and `sendTransaction` are forbidden as alternative signing paths
- The underlying `@morpho-org/cli` itself is unsigned-by-design: its safety doc says "Never sign or broadcast — unsigned payloads only"

## Local development

```bash
cd ~/.agents/skills/morpho-cli-onchainos
npm install
npx tsx cli.ts --help
npx tsx cli.ts healthCheck '{}'
npx tsx cli.ts queryVaults '{"chain":"base","limit":3}'
```

By default the skill runs in MOCK mode (canned fixtures). To hit the real Morpho CLI:

```bash
MORPHO_RUNTIME=real npx tsx cli.ts queryVaults '{"chain":"base","limit":3}'
```

(Real mode shells out to `npx @morpho-org/cli@latest`. The CLI is downloaded on first run.)
