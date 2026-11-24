## Overview

DigiFT is a regulated RWA (Real-World Asset) tokenization platform. This plugin
enables AI agents to query on-chain and off-chain DigiFT product data — products,
prices, fees, settlement calendars, whitelist status — and build unsigned
subscribe/redeem/approve transactions routed through Onchain OS for secure TEE-based signing.

Core operations:

- List available RWA products with tradability flags
- Query real-time indicative prices and historical price data
- Check subscription/redemption parameters (fees, min, max, increment)
- View trading calendars and settlement schedules
- Check wallet whitelist status (KYC verification)
- Look up order status and order history
- Build unsigned subscribe, redeem, and approve transactions (TxBody)
- Route transactions through Onchain OS for TEE-based signing

Tags: `rwa` `digift` `tokenization` `stablecoin` `defi` `subscription`

## Prerequisites

- DigiFT API key (`DIGIFT_API_KEY`) — required for all queries
- Onchain OS agentic wallet — for on-chain transaction signing
- Supported chains: Ethereum (1), BSC (56), Polygon (137), Monad (143),
  Mantle (5000), Plume (98867), Arbitrum (42161)
- Supported currencies: USDC and other platform-supported currencies

## Quick Start

1. **Set up environment**: `export DIGIFT_API_KEY=<your-key>`
2. **Browse products**: `digift products` — lists all available RWA tokens
3. **Check a product**: `digift price <tokenCode>` — current price and yield
4. **Build a subscription**: `digift subscribe --product KEY --amount N --from ADDR --chain ID`
5. **Sign and broadcast**: Agent returns a `pending_sign` object — Onchain OS handles the rest
6. **Track your order**: `digift order <txHash>` — check settlement status
