# Agent Risk Firewall

Pre-trade risk firewall for OKX Agentic Wallet swaps on X Layer and Solana.

This directory contains the OKX Plugin Store package for `agent-risk-firewall`: a Skill + Python CLI that helps an AI agent evaluate a proposed swap, token trade, or approval before asking the user to sign.

The plugin returns a deterministic JSON verdict:

- `allow`: continue the normal signing flow if the user already requested it
- `warn`: show the warning reasons and require explicit confirmation
- `block`: cancel the operation and do not request a signature

## Safety Boundary

Agent Risk Firewall does not sign transactions, broadcast transactions, execute swaps, revoke approvals, or handle private keys. It only evaluates intent, quote, token context, and optional transaction context.

If OKX OnchainOS scan or simulation data is unavailable, the plugin treats verification as incomplete. In `balanced`, `competition`, and `degen-small-size`, incomplete verification returns at least `warn`; in `strict`, it returns `block`.

## What It Checks

- X Layer and Solana address/chain compatibility
- Token risk from OKX OnchainOS token scan
- Transaction risk from OKX OnchainOS tx-scan
- Simulation failure or revert from OKX OnchainOS gateway simulation
- Slippage and price impact thresholds
- Per-trade and wallet-exposure caps
- Low liquidity when liquidity data is available
- Optional external evidence from GoPlus, Birdeye, and RootData
- Approval spender and unlimited allowance risk
- Audit trail fields: `decisionId`, `policyVersion`, `evidenceHash`

## Commands

```bash
agent-risk-firewall check --input request.json --format json
agent-risk-firewall policy --profile balanced
agent-risk-firewall policy --profile strict
agent-risk-firewall policy --profile competition
agent-risk-firewall policy --profile degen-small-size
agent-risk-firewall self-test
```

Use `--input -` to read JSON from stdin.

## Input Contract

```json
{
  "chain": "xlayer",
  "operation": "swap",
  "walletAddress": "0x0000000000000000000000000000000000000000",
  "tokenIn": {
    "address": "0x0000000000000000000000000000000000000001",
    "symbol": "USDC",
    "decimals": 6
  },
  "tokenOut": {
    "address": "0x0000000000000000000000000000000000000002",
    "symbol": "TOKEN",
    "decimals": 18
  },
  "amountIn": "100",
  "amountInUsd": 100,
  "quote": {
    "expectedOut": "12345",
    "slippagePct": 1,
    "priceImpactPct": 0.8,
    "route": ["OKX DEX"],
    "venue": "okx-dex"
  },
  "tx": {
    "from": "0x0000000000000000000000000000000000000000",
    "to": "0x0000000000000000000000000000000000000003",
    "data": "0x",
    "value": "0"
  },
  "approval": {
    "spender": "0x0000000000000000000000000000000000000004",
    "spenderType": "contract",
    "isUnlimited": false
  },
  "externalEvidence": {
    "goplus": {},
    "birdeye": {},
    "rootdata": {}
  },
  "policyProfile": "balanced"
}
```

Supported values:

- `chain`: `xlayer`, `solana`
- `operation`: `buy`, `sell`, `swap`, `approval`
- `policyProfile`: `balanced`, `strict`, `competition`, `degen-small-size`

## Policy Profiles

| Profile | Use case | Key behavior |
| --- | --- | --- |
| `balanced` | Default retail agentic trading guardrail | Blocks critical risk and warns on elevated risk. |
| `strict` | High-safety mode | Blocks unavailable scans, token `HIGH`, EOA spenders, and tighter slippage/price-impact limits. |
| `competition` | OKX Agentic Trading style workflows | Tighter caps and blocks stablecoin/native-only pairs. |
| `degen-small-size` | Small meme-token experiments | Higher slippage tolerance with a hard 25 USD trade cap. |

## Balanced Policy

| Signal | Result |
| --- | --- |
| Token `CRITICAL` on buy/swap | `block` |
| Token `CRITICAL` on sell | `warn` |
| Token `HIGH` or `MEDIUM` | `warn` |
| OKX tx-scan `block` | `block` |
| OKX tx-scan `warn` | `warn` |
| Simulation revert/failure | `block` |
| Scan or simulation timeout/unavailable | `warn` |
| Slippage `2%` to `5%` | `warn` |
| Slippage above `5%` | `block` |
| Price impact `3%` to `8%` | `warn` |
| Price impact above `8%` | `block` |
| Trade above `250 USD` | `block` |
| Trade above `10%` of wallet value, when wallet value is provided | `block` |

## Local Test

From the repository root:

```powershell
$env:PYTHONDONTWRITEBYTECODE = "1"
python -m pytest .\skills\agent-risk-firewall\tests -q -p no:cacheprovider
& "$env:USERPROFILE\.local\bin\plugin-store.exe" lint .\skills\agent-risk-firewall
```

Expected results:

```text
28 passed
Plugin 'agent-risk-firewall' passed all checks
```

## Agent Integration Pattern

```text
1. Agent receives a natural-language trade intent.
2. Agent gets quote or unsigned transaction context from OKX OnchainOS.
3. Agent runs agent-risk-firewall check.
4. If allow, continue only if the user requested execution.
5. If warn, show reasons and require explicit confirmation.
6. If block, cancel and do not ask the user to sign.
```

Compatible strategy pattern:

```text
xlayer-alpha-hunter -> unsigned swap -> agent-risk-firewall -> execute only if allowed
smart-tradex -> quote/tx context -> agent-risk-firewall -> warn/block gate
otto-alpha-sniper -> external evidence + tx context -> agent-risk-firewall -> final confirmation
```

## Disclaimer

This plugin is a defensive guardrail, not a guarantee of safety. On-chain data, scan results, and simulations can be incomplete, stale, or wrong. Trading and DeFi activity can cause loss of funds. Use dry-run mode first and start with small amounts.

## License

MIT
