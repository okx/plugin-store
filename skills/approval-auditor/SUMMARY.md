# approval-auditor — Summary

## 1. Overview

`approval-auditor` is a read-only security skill that audits the ERC-20
token approvals on an EVM wallet. Given a wallet address and a supported
chain, it enumerates every active allowance, risk-scores each one, and
produces a prioritized list of approvals the user should revoke — with
ready-to-submit `approve(spender, 0)` calldata attached to each entry.

The skill exists because forgotten and unlimited token approvals are one
of the most common sources of irreversible Web3 loss. A single old
approval to a later-exploited protocol, or a rushed approval to a
phishing contract, can drain a wallet silently. This audit gives the
agent a concrete way to answer: "which of my approvals are dangerous,
and in what order should I revoke them?"

The skill is strictly read-only. It does not sign, send, or broadcast
any transaction. It does not access private keys, seed phrases, or
signed transaction data. Revocation is performed by the user in their
own wallet, using the calldata the script produces.

Supported chains: Ethereum, Arbitrum, Optimism, Base, Polygon, BSC,
Avalanche.

## 2. Prerequisites

1. **Python 3.10 or newer** available as `python3`.
2. **A single Python dependency**, `requests==2.32.3`, installed from
   the plugin's `requirements.txt`:
   ```
   python3 -m pip install --user -r requirements.txt
   ```
3. **An Etherscan V2 API key**, obtained by the user at
   `https://etherscan.io/apis`. The free tier (5 requests per second,
   100,000 requests per day) is sufficient for any realistic wallet.
   The user exports the key in their own terminal:
   ```
   export ETHERSCAN_API_KEY=<their-key>
   ```
   The key is read only from the environment variable. The agent never
   sees it, and the key must not be pasted into the chat.
4. **A valid EVM wallet address** (0x-prefixed, 42 characters, hex only).

No other credentials, wallets, or external accounts are required.

## 3. Quick Start

From the plugin directory, with the API key exported:

```
python3 scripts/audit.py \
    --address 0xd8da6bf26964af9d7eed9e03e53415d37aa96045 \
    --chain ethereum
```

The script prints a human-readable report wrapped in `<external-content>`
boundary markers, one block per active approval, sorted by risk score
descending. Each block shows:

- Risk band (CRITICAL / HIGH / MEDIUM / LOW) and numeric score (0–100)
- Token symbol and contract address
- Spender label (from a curated list of known protocols) or an explicit
  "UNKNOWN-EOA" / "unknown-contract" flag
- Current allowance (or `UNLIMITED` if effectively infinite)
- Days since the approval was last modified
- Human-readable reasons for the score
- `approve(spender, 0)` calldata the user can paste into their wallet

For machine-parseable output, pass `--format json`. For a specific chain,
pass `--chain arbitrum` / `optimism` / `base` / `polygon` / `bsc` /
`avalanche`.

### What to do with the output

The report is ordered to be acted on top-down:

- **CRITICAL (score ≥ 80)** — revoke immediately. The common pattern is
  an unlimited allowance whose spender is an externally-owned account
  rather than a contract. This is almost always a phishing drain.
- **HIGH (50–79)** — unknown-contract spenders with unlimited allowances,
  or long-dormant approvals on blue-chip assets. Revoke unless the
  spender can be positively identified as a legitimate protocol.
- **MEDIUM (30–49)** — unknown spenders with bounded allowances or
  dormant known-protocol approvals. Review and consider revoking.
- **LOW (0–29)** — recent, bounded approvals to known protocols.
  Informational only.

To revoke an entry: open the user's wallet, create a new transaction
with `to` set to the token contract address and `data` set to the entry's
`revoke_calldata`. Signing and broadcasting is the user's action — the
skill never does this.

### Coverage caveat

The skill detects allowances granted via the standard ERC-20 `approve()`
function (which emits an on-chain `Approval` event). Allowances granted
via EIP-2612 `permit()` or via Permit2 off-chain signatures may not be
visible through this path. Treat the audit as a lower bound on exposure.
