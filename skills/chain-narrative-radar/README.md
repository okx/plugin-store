# Chain Narrative Radar

Contest-ready Strategy Skill adapted from `chain_narrative_radar.py`.

It uses public narrative and safety data to produce a watchlist, then only trades liquid symbols that are supported by Hyperliquid. All execution is delegated to `hyperliquid-plugin` with `--strategy-id chain-narrative-radar`.

## Contest role

- Target basic skill: Hyperliquid Plugin
- Primary ranking target: trade count
- Public mode: no Telegram secrets, no local runtime database, no direct wallet handling

## Files

- `plugin.yaml` - Plugin Store metadata
- `.claude-plugin/plugin.json` - Claude Skill metadata
- `SKILL.md` - strategy instructions
- `SUMMARY.md` - short listing summary
- `LICENSE` - MIT license

