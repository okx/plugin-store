# Otto Alpha Sniper

A Strategy Skill for the [OKX Onchain OS Plugin Store](https://github.com/okx/plugin-store). Natural-language intent → live Hyperliquid perpetual trade, driven by Otto AI's multi-source alpha signals.

## What it does

User tells their AI agent something like *"open a perp on the strongest altcoin"* or *"follow the KOLs on ETH"*. The Skill:

1. Checks the user's Hyperliquid account via the Hyperliquid Basic Skill (funding + signing ready?).
2. Pulls a live signal from Otto AI's public signals feed (trending momentum, KOL sentiment, or funding-rate extremes).
3. Picks the highest-conviction coin + direction.
4. Confirms with the user (explicit prompt — no silent orders).
5. Places the perp via `hyperliquid-plugin order` with auto TP/SL bracket via `hyperliquid-plugin tpsl`.
6. Returns a compact trade card.

Dry-run is the default. Live orders require `--confirm`.

## Install

```bash
npx skills add okx/plugin-store --skill hyperliquid-plugin --yes --global
npx skills add okx/plugin-store --skill otto-alpha-sniper --yes --global
```

## Requirements

- onchainos CLI ≥ 2.0.0
- USDC on Arbitrum (for deposit into Hyperliquid)
- ETH on Arbitrum (gas for the deposit)
- A registered Hyperliquid signing address (`hyperliquid-plugin register`)

## Files

| File | Purpose |
|---|---|
| [SKILL.md](SKILL.md) | Agent protocol — what the AI reads to orchestrate a trade |
| [SUMMARY.md](SUMMARY.md) | User-facing overview |
| `plugin.yaml` | Plugin Store manifest |
| `scripts/config.py` | Hot-reload parameters (thresholds, caps, risk limits) |
| `scripts/bot.py` | Optional autonomous poller mode |

## Status

Submission target: OKX Plugin Store Developer Challenge Season 1 (snapshot 2026-05-07).

## License

MIT
