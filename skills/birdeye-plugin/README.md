# birdeye-plugin

Birdeye multi-chain DeFi analytics plugin with dual live access mode:

- `apikey`: standard Birdeye API with `X-API-KEY`
- `x402`: pay-per-request Birdeye API (`/x402`) using Solana USDC
- `auto`: use `apikey` when available, otherwise `x402`

## Runtime Notes

- `apikey` mode can run on lower Node versions.
- `x402` mode requires Node.js 20+.
- If you see `No random values implementation could be found`, switch to Node 20 and retry.

## Requirements

- For `apikey` mode: `BIRDEYE_API_KEY`
- For `x402` mode: `SOLANA_PRIVATE_KEY` (base58 private key), wallet funded with USDC on Solana mainnet

## Supported commands

- `node runtime/dist/index.js price --address <TOKEN> --chain solana`
- `node runtime/dist/index.js trending --chain solana --limit 20`
- `node runtime/dist/index.js overview --address <TOKEN> --chain solana`
- `node runtime/dist/index.js security --address <TOKEN> --chain solana`

## Environment

### API key mode

- `export BIRDEYE_MODE=apikey`
- `export BIRDEYE_API_KEY=your_key`

### x402 mode

- `export BIRDEYE_MODE=x402`
- `export SOLANA_PRIVATE_KEY='your_base58_private_key'`

### Auto mode

- `export BIRDEYE_MODE=auto`

Behavior:
- If `BIRDEYE_API_KEY` exists -> `apikey`
- Else if `SOLANA_PRIVATE_KEY` exists -> `x402`
- Else -> configuration error

## Notes

- Base API: `https://public-api.birdeye.so`
- x402 API: `https://public-api.birdeye.so/x402`
- Some Birdeye endpoints may not be available on x402. Use `apikey` mode as fallback where needed.
