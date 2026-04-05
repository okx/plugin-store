# polymarket-agent-skills
Full-stack Polymarket prediction market integration covering order placement, market data, WebSocket streaming, cross-chain bridge, and gasless transactions on Polygon.

## Highlights
- L1 (EIP-712) and L2 (HMAC-SHA256) authentication with builder header support
- Order placement: GTC, GTD, FOK, FAK, batch, post-only, and heartbeat orders
- CLOB API and Gamma/Data API market data (orderbook reads require no auth)
- WebSocket streaming across market, user, and sports channels
- CTF split/merge/redeem and negative risk market operations
- Cross-chain bridge: deposits and withdrawals
- Gasless relayer transactions
- TypeScript (`@polymarket/clob-client`) and Python (`py-clob-client`) SDK examples
