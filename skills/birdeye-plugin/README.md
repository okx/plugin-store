# birdeye-plugin

Birdeye multi-chain DeFi analytics plugin with dual live access mode:

- `apikey`: standard Birdeye API with `X-API-KEY` (full endpoint coverage in this plugin)
- `x402`: pay-per-request Birdeye API (`/x402`) using Solana USDC (x402-supported subset)
- `auto`: use `apikey` when available, otherwise `x402`

## Runtime Notes

- `apikey` mode can run on lower Node versions.
- `x402` mode requires Node.js 20+.
- If you see `No random values implementation could be found`, switch to Node 20 and retry.

## Requirements

- For `apikey` mode: `BIRDEYE_API_KEY`
- For `x402` mode: key file `~/.birdeye/key` (base58 private key, mode 0600), wallet funded with USDC on Solana mainnet

## Commands

- `node runtime/dist/index.js list [--mode apikey|x402]`
- `node runtime/dist/index.js call --endpoint <key> --chain solana --param value ...`
- Backward-compatible aliases:
  - `node runtime/dist/index.js price --address <TOKEN> --chain solana`
  - `node runtime/dist/index.js trending --chain solana --limit 20`
  - `node runtime/dist/index.js overview --address <TOKEN> --chain solana`
  - `node runtime/dist/index.js security --address <TOKEN> --chain solana`

## Coverage Policy

- `apikey` mode: full registry defined in runtime endpoint map.
- `x402` mode: restricted to endpoints supported by bd-x402/x402 routes.
- If an endpoint is unavailable in `x402`, switch to `apikey` mode.
