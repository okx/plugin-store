# Bluefin DEX Plugin Design

## Research Findings

### Chain Support
Bluefin is **Sui-only**. Key history:
- v1 (Firefly Protocol) was on Arbitrum EVM — now archived/deprecated
- v2 current production is exclusively on **Sui** blockchain
- The `fireflyprotocol/bluefin-exchange-contracts-evm` repo exists but is v1.0.0 (May 2023) legacy, no longer active
- All current infrastructure uses Sui fullnode RPC

### Implementation Decision
Since Sui is not supported by onchainos CLI for write transactions, this plugin implements:
- **Read operations**: Full live data via Bluefin REST API
- **Write operations**: Preview/dry-run mode with `--confirm` flag (explains what tx would be submitted)

### API Endpoints
Base URL: `https://dapi.api.sui-prod.bluefin.io`

Key endpoints used:
- `GET /exchangeInfo` — all trading pairs, contract addresses, market configs
- `GET /ticker` — 24h price stats for all markets or a specific symbol
- `GET /marketData` — market statistics
- `GET /marketData/symbols` — list of all symbols
- `GET /userPosition?symbol=&userAddress=` — open positions for wallet
- `GET /orderbook?symbol=` — order book depth
- `GET /orders?address=` — open orders for wallet
- `GET /account?address=` — account balance and margin info
- `GET /fundingRate?symbol=` — current funding rate

### Sui Program / Package IDs
From `fireflyprotocol/bluefin-v2-client-ts` constants:
- Sui objects are referenced by package IDs, not EVM contract addresses
- Main exchange package handles perpetual trading via Sui Move modules

### Main Operations
1. **markets** — list all available trading markets with prices (read-only, no auth)
2. **ticker** — 24h stats for a specific market
3. **positions** — show open perpetual positions for a wallet (read-only)
4. **quote** — get orderbook depth / quote for a given size
5. **open-position** — preview opening a perpetual position (write, preview without --confirm)
6. **close-position** — preview closing a perpetual position (write, preview without --confirm)

### Notes
- All write operations display a detailed transaction preview
- Sui write path requires Sui wallet + SDK, outside onchainos scope
- API authentication (JWT via /authorize) required for user-specific endpoints
- Public endpoints (ticker, exchangeInfo, orderbook) require no auth
