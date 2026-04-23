# polymarket-btc-5min-momentum

> A demonstration strategy that bets BTC 5-minute Up/Down prediction markets on Polymarket based on recent short-term BTC price momentum. Routes every order through `polymarket-plugin` with strategy attribution.

## Quickstart

1. Install the dependent plugin (one-time):

    ```bash
    npx skills add okx/plugin-store --skill polymarket-plugin --yes --global
    ```

2. Make sure `polymarket-plugin` is configured with an onchainos wallet that has USDC.e on Polygon (see `polymarket-plugin/SKILL.md`).

3. Preview the next 5-min slot's decision without trading:

    ```bash
    python3 strategy.py --dry-run
    ```

4. Place a single bet:

    ```bash
    python3 strategy.py --amount 2.5
    ```

5. Run the loop (one decision per 5-min UTC boundary):

    ```bash
    python3 strategy.py --loop --amount 2.5
    ```

See `SKILL.md` for the full flag reference, strategy rationale, attribution detail, and known limitations.

## Files

| File | Purpose |
|------|---------|
| `plugin.yaml` | Plugin Store manifest. `category: strategy` + `dependent_plugin: polymarket-plugin`. |
| `strategy.py` | Main orchestrator. Stdlib only — no pip installs required. |
| `SKILL.md` | Full documentation surfaced to Claude/Cursor/OpenClaw. |
| `SKILL_SUMMARY.md` | Short registry summary. |
| `LICENSE` | MIT. |

## License

MIT — see `LICENSE`.
