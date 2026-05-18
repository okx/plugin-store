# across-protocol-onchainos

OnchainOS-routed integration of [Across Protocol](https://across.to) — crosschain bridging and any-to-any token swaps via the Across Swap API. Routes signing through Onchain OS in a TEE instead of inline wallet calls.

Form: **B-REST** (v1.14 taxonomy) — wraps `app.across.to/api`.

Supported chains: ethereum, arbitrum, base, optimism, polygon, zksync, linea (and growing — `listSupportedChains` returns the live list).

## Install

```bash
npx skills add <your-org>/across-protocol-onchainos
```

Or via local copy:

```bash
mkdir -p ~/.agents/skills
cp -R . ~/.agents/skills/across-protocol-onchainos
ln -sfn ~/.agents/skills/across-protocol-onchainos ~/.claude/skills/across-protocol-onchainos
```

## First-time authentication

All on-chain signing is handled by Onchain OS inside a local TEE. This DApp never touches private keys or persists login state.

```bash
onchainos wallet login user@example.com
onchainos wallet verify <6-digit-code>
onchainos wallet status
```

## Tools

| Tool | Class | What it does |
|---|---|---|
| `listSupportedChains` | Read | Returns the chains Across currently supports (origin + destination) |
| `getQuote` | Read | Fetches a swap quote from `/swap/approval` without committing to a route |
| `getDepositStatus` | Read | Polls `/deposit/status` for the fill state of an existing deposit |
| `buildSwap` | Tx | Calls `/swap/approval` and reshapes the response into a `pending_sign` envelope (may include an ERC-20 approve as a Step 1 of 2) |

## Usage example

> User: Bridge 100 USDC from Arbitrum to Base

The Agent:

1. Calls `getQuote({inputToken, outputToken, originChainId: 42161, destinationChainId: 8453, amount: "100000000"})` for a preview
2. Calls `buildSwap({...same args, walletAddress})` to get a `pending_sign` envelope
3. Routes the broadcast via `onchainos wallet contract-call --chain arbitrum --to <to> --input-data <data> --amt <value>`
4. Onchain OS signs + broadcasts inside the TEE and returns `txHash`
5. (Optional) Polls `getDepositStatus({depositTxnRef: <txHash>, originChainId: 42161})` until the destination-chain fill confirms

## Local smoke

```bash
npm install
npx tsx cli.ts --version
npx tsx cli.ts --help

# Mock mode (default — canned data, no network)
npx tsx cli.ts listSupportedChains '{}'
npx tsx cli.ts getQuote '{"inputToken":"0xaf88d065e77c8cC2239327C5EDb3A432268e5831","outputToken":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","amount":"10000000","originChainId":42161,"destinationChainId":8453}'

# Real mode (live REST against app.across.to/api)
ACROSS_RUNTIME=real npx tsx cli.ts getQuote '{"inputToken":"0xaf88...","outputToken":"0x833...","amount":"10000000","originChainId":42161,"destinationChainId":8453}'
```

## Security

- This DApp never reads, stores, or transmits the user's private key, seed phrase, or keystore
- All `pending_sign` transactions are signed exclusively through Onchain OS
- `ethers.Wallet`, `signTransaction`, `sendTransaction` are forbidden
- The upstream's example `wallet.sendTransaction(...)` patterns in `swap/SKILL.md` are markdown-only documentation (not executable code in this artifact); they are explicitly **replaced** by the `pending_sign` envelopes produced by `buildSwap` and routed through `onchainos wallet contract-call`
