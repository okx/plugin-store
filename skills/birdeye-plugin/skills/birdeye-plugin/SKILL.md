---
name: birdeye-plugin
description: Birdeye DeFi analytics with dual live access mode (apikey full coverage, x402 supported subset).
---

# Birdeye Plugin Skill

Use this skill for Birdeye token/market/holder/smart-money/trader data.

## Runtime Requirement Notes

- `apikey` mode can run on lower Node versions.
- `x402` mode requires Node.js 20+.
- If `BIRDEYE_MODE=x402` (or `auto` resolves to x402), verify `node -v` and ensure major version is `>= 20`.

## Modes

- `apikey`: full endpoint coverage in plugin registry.
- `x402`: x402-supported subset only.
- `auto`: prefer `apikey`, fallback to `x402`.

## Environment

- `BIRDEYE_MODE=auto|apikey|x402`
- `BIRDEYE_API_KEY=...` (apikey mode)
- `SOLANA_PRIVATE_KEY=...` base58 (x402 mode)

## Commands

- `node runtime/dist/index.js list [--mode apikey|x402]`
- `node runtime/dist/index.js call --endpoint <key> --chain <chain> --param value ...`

Aliases:
- `price`, `trending`, `overview`, `security`

## Routing Guidance

1. Run `list` for active mode when uncertain.
2. Use `call --endpoint <key>` for any supported endpoint.
3. If endpoint unavailable in `x402`, switch to `apikey` mode.
