# approval-auditor

Read-only audit of ERC-20 token approvals on an EVM wallet. Produces a
prioritized, risk-scored revocation list with ready-to-submit calldata.

## Install (via Plugin Store)

```
npx skills add okx/plugin-store --skill approval-auditor
```

## Install dependencies

```
python3 -m pip install --user -r requirements.txt
```

Only runtime dependency: `requests==2.32.3`.

## Usage

```
export ETHERSCAN_API_KEY=<your-etherscan-v2-key>
python3 scripts/audit.py --address 0x... --chain ethereum
```

Supported chains: `ethereum`, `arbitrum`, `optimism`, `base`, `polygon`,
`bsc`, `avalanche`.

Full documentation: see `SKILL.md` for agent-facing specification and
`SUMMARY.md` for a user-facing summary.

## How it works

1. Calls the Etherscan V2 multichain API (`api.etherscan.io`) with the
   supplied API key and chain ID.
2. Fetches all ERC-20 `Approval` event logs indexed on the target
   wallet.
3. Deduplicates logs to one record per `(token, spender)` pair, keeping
   the latest by block number.
4. For each pair, queries the live `allowance(owner, spender)` value via
   `eth_call`. Entries with zero current allowance are dropped.
5. For each spender, queries `eth_getCode` to classify as contract or
   externally-owned account.
6. Scores each remaining approval using `risk_model.py` — a
   deterministic, pure-Python function with documented scoring weights.
7. Sorts results by score descending and emits either a text report or
   a JSON object.

The script never signs or broadcasts anything. The only external domain
contacted is `api.etherscan.io`, which is declared in `plugin.yaml`.

## Repository layout

```
approval-auditor/
├── .claude-plugin/
│   └── plugin.json            Claude skill registration
├── LICENSE                    MIT
├── plugin.yaml                Plugin Store manifest
├── README.md                  This file
├── requirements.txt           Pinned Python dependency
├── scripts/
│   ├── audit.py               CLI entry point
│   ├── known_spenders.py      Curated whitelist of known protocols
│   └── risk_model.py          Deterministic risk scoring
├── SKILL.md                   Agent-facing skill specification
└── SUMMARY.md                 User-facing summary
```

## License

MIT. See `LICENSE`.
