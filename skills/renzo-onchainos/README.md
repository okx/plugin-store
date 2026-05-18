# renzo-onchainos

OnchainOS-routed Renzo Protocol integration — liquid restaking ezETH on Ethereum mainnet via the RestakeManager + WithdrawQueue contracts. Read-only enrichment from the public Renzo app API.

Supported chains: `ethereum` (mainnet only in v0.1.0)

## Install

```bash
npx skills add <your-org>/renzo-onchainos
```

This skill requires Onchain OS. On first use, the LLM will automatically run the install command in `## Pre-flight Checks`:

```bash
npx skills add okx/onchainos-skills
```

> Once published to the OKX Skills Marketplace, end users can install with a single `npx skills add` command. Until then, install via `git clone` or `cp -r` into `~/.agents/skills/renzo-onchainos/`.

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

## Usage examples

Trigger tools with natural language in your Agent:

> User: "Deposit 0.5 ETH into Renzo to mint ezETH"
> User: "What's the current Renzo ezETH APR?"
> User: "Show me my pending Renzo withdrawal requests for 0xMyWallet"
> User: "Queue a withdrawal of 1 ezETH for stETH" (or for the ETH sentinel `0xEeE...`)

The Agent follows this flow for a write call:

1. Calls the DApp tool to construct `unsigned_tx` (e.g. `buildDepositEth({amount: '0.5'})`)
2. Tool returns `pending_sign` + `next_action.tool = 'onchainos wallet contract-call'`
3. Agent routes to Onchain OS `onchainos wallet contract-call`
4. Onchain OS signs + broadcasts inside the TEE and returns `txHash`

For `buildRequestWithdraw`, if the user hasn't approved ezETH to the WithdrawQueue yet, the tool emits an approve `pending_sign` first (Step 1 of 2). The Agent broadcasts the approve, waits for confirmation, then re-invokes `buildRequestWithdraw` with the same params — the tool re-reads on-chain allowance and emits the withdraw `pending_sign` (Step 2 of 2).

## Local smoke

```bash
npm install
npx tsx cli.ts --version
npx tsx cli.ts --help
npx tsx cli.ts getApr '{}'                                      # mock returns canned data
RENZO_RUNTIME=real npx tsx cli.ts getApr '{}'                   # real REST call
RENZO_RUNTIME=real npx tsx cli.ts getEzethRate '{"source":"chain"}'   # real viem call
npx tsx cli.ts buildDepositEth '{"amount":"0.5"}'               # pending_sign envelope, no RPC needed
```

## Source provenance

- **Contracts**: ABIs and addresses verified against [`Renzo-Protocol/contracts-public@master`](https://github.com/Renzo-Protocol/contracts-public) and [`docs.renzoprotocol.com/docs/contracts/ethereum-mainnet`](https://docs.renzoprotocol.com/docs/contracts/ethereum-mainnet). For production, pin to a specific commit SHA.
- **Read API**: `app.renzoprotocol.com/api/{apr,stats}` — public, no auth, read-only. (Note: there is no `api.renzoprotocol.com` host; the API is hosted under `app.renzoprotocol.com/api/*`.)
- **No SDK**: Renzo does not publish a TypeScript SDK with calldata builders. v0.1.0 builds calldata directly with viem against the verified ABIs (Form C scaffold path).

## Security

- This DApp never reads, stores, or transmits the user's private key, seed phrase, or keystore
- All `pending_sign` transactions are signed exclusively through Onchain OS
- `ethers.Wallet`, `signTransaction`, and `sendTransaction` are forbidden as alternative signing paths
