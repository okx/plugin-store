# liqgrid — Plugin Store Skill

Natural-language perpetual grid strategy on Hyperliquid, powered by a
deterministic TypeScript engine.

## Install

```
npx skills add okx/plugin-store --skill liqgrid
```

## Requires

- `onchainos` CLI + unlocked Agentic Wallet
- `hyperliquid-plugin` (basic plugin) installed separately:
  `npx skills add okx/plugin-store --skill hyperliquid-plugin`
- ≥ 20 USDC on your Hyperliquid perp account

## Usage

Once installed, talk to any Onchain-OS agent in natural language:

> "BTC 90k to 95k, balanced grid, $500 at 2x"

The agent will:
1. Fetch BTC market data via the Hyperliquid basic plugin.
2. Run `liqgrid plan` (the compiled binary shipped with this Skill) to
   compute a deterministic grid.
3. Show the DRY-RUN plan with grid count, stop-loss, max loss, and
   expected fills/day.
4. Execute only after you confirm — through the Hyperliquid basic plugin.

## Safety (hardcoded in the binary)

- Dry-run default
- Max $5,000 / 10× / 50 rungs per grid
- Max loss at range break: 30% of notional (warning enforced)
- No private key handling — all signing via Agentic Wallet TEE
- Binary makes no network calls (`api_calls: []`)

## Source

The binary is TypeScript, compiled to JS, distributed via `bun install -g`
by the Plugin Store CI. Source:
https://github.com/dddd86971-cloud/liqgrid

See `SKILL.md` for the full command reference and Security Notices.

## License

MIT — see `LICENSE`.

## Season 1 Developer Challenge

This Skill is submitted to the OKX Onchain OS Plugin Store Season 1
Developer Challenge (2026-04-23 → 2026-05-07). All on-chain writes flow
through the Hyperliquid basic plugin as required by challenge eligibility
rules.
