# Otto KOL Follow

Mirror the top 50 crypto KOLs on Twitter/X into a Hyperliquid perpetual — one consensus-driven trade per user intent, with automatic TP/SL brackets, inside OKX Onchain OS Agentic Wallet.

**Dry-run by default.** `--confirm` required for live orders.

## Files

```
otto-kol-follow/
├── .claude-plugin/
│   └── plugin.json       # Claude Skill registration metadata
├── plugin.yaml           # Plugin Store manifest
├── SKILL.md              # AI agent protocol (reactive 7-step flow)
├── SUMMARY.md            # User-facing summary
├── README.md             # This file
├── LICENSE               # MIT
└── scripts/
    ├── config.py         # Hot-reloadable params (edit this, not bot.py)
    └── bot.py            # Optional autonomous poller (advanced users)
```

## Decision rule

Fire a trade only if:

- Cohort sample size ≥ `MIN_KOL_COUNT` (default 40) — prevents thin-sample mirrors
- Otto confidence ≥ `MIN_CONFIDENCE_KOL` (default 0.70)
- Direction is not `flat` — no trade on a split cohort

Otherwise the Skill aborts with a clear reason and does NOT place.

## Leverage philosophy

KOL consensus is a lagging, reflexive signal that can be wrong at tops and bottoms. The Skill caps leverage at 3x by design. Higher leverage degrades risk-adjusted returns on this strategy class.

## Sibling Skills

This Skill shares the Otto AI signal-feed data moat with:

- **otto-alpha-sniper** — multi-mode sniper (trending / kol-follow / funding-fade)
- **otto-mispricing-assistant** — Polymarket near-resolution mispricing scanner

## Links

- Otto AI: https://useotto.xyz
- Signal feed contract: [../SIGNAL_FEED_CONTRACT.md](../SIGNAL_FEED_CONTRACT.md)
- Docs: https://docs.useotto.xyz

## License

MIT — see `LICENSE`.
