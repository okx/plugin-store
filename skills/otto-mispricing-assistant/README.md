# Otto Mispricing Assistant

Scan near-resolution Polymarket markets where the implied probability diverges from Otto AI's live news, KOL-sentiment, and funding-rate signals — surface ranked candidates, trade one market at a time with explicit user confirmation.

**This is a scanner, not a bot.** No batch execution. No autonomous trading. Every trade requires the user to type "confirm".

## Files

```
otto-mispricing-assistant/
├── .claude-plugin/
│   └── plugin.json       # Claude Skill registration metadata
├── plugin.yaml           # Plugin Store manifest
├── SKILL.md              # AI agent protocol (9-step reactive flow)
├── SUMMARY.md            # User-facing summary
├── README.md             # This file
├── LICENSE               # MIT
└── scripts/
    ├── config.py         # Hot-reloadable params
    └── bot.py            # Optional notify-only scanner (NEVER trades)
```

## How it works

1. Pull Otto's live signals: news-flash (last 6h, severity ≥ 3), KOL sentiment (24h window), funding extremes (top 5).
2. List active Polymarket markets in crypto / macro / elections, filter by resolution window + liquidity + volume.
3. For each market, derive an Otto probability estimate from matched signals vs. the implied probability (YES-token mid).
4. Rank by `|edge| × signal_confidence × liquidity_score`. Keep top 5 above the 8pp edge threshold.
5. Present the ranked list. User picks one or "none".
6. Re-quote the chosen market right before confirmation — prices move fast near resolution.
7. User types "confirm". Skill places a single `polymarket-plugin buy` with `--strategy-id` attribution.
8. Report back. Remind the user that mispricings can persist or widen.

## Budget enforcement

- Per-trade cap: **$50 USDC.e** (hard, no user override).
- Per-session cap: **$200 USDC.e** cumulative.
- Any user request exceeding these is sized down with a warning, or refused if the session cap is spent.

## Sibling Skills

Shares the Otto AI signal feed with:

- **otto-alpha-sniper** — Hyperliquid perp sniper (trending / kol-follow / funding-fade)
- **otto-kol-follow** — Hyperliquid KOL-consensus mirror
- **otto-macro-cross-venue** (Tier 2) — fires HL + Polymarket as matched-pair trades on macro flashes

## Links

- Otto AI: https://useotto.xyz
- Signal feed contract: [../SIGNAL_FEED_CONTRACT.md](../SIGNAL_FEED_CONTRACT.md)
- Polymarket Basic Skill: https://github.com/okx/plugin-store/tree/main/skills/polymarket-plugin
- Docs: https://docs.useotto.xyz

## License

MIT — see `LICENSE`.
