# birdeye-plugin

Birdeye multi-chain DeFi analytics plugin with dual mode access:

- `apikey`: standard Birdeye API with `X-API-KEY`
- `x402`: pay-per-request Birdeye API (`/x402`) using Solana USDC
- `auto`: use `apikey` when available, otherwise `x402`

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
- `export SOLANA_PRIVATE_KEY='[1,2,3,...]'`

Notes:
- Wallet must have USDC on Solana mainnet.
- Not all endpoints are available in x402 mode.

### Auto mode

- `export BIRDEYE_MODE=auto`

Behavior:
- If `BIRDEYE_API_KEY` exists -> `apikey`
- Else if `SOLANA_PRIVATE_KEY` exists -> `x402`
- Else -> configuration error
