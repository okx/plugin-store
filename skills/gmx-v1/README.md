# GMX V1 Plugin

Trade perpetuals, swap tokens, and manage GLP liquidity on GMX V1 (Arbitrum/Avalanche).

## Features

- **get-prices**: Fetch current oracle prices for all GMX V1 tokens
- **get-positions**: View open perpetual positions for any wallet
- **swap**: Swap ERC-20 tokens via GMX V1 Router (no execution fee)
- **buy-glp**: Mint GLP tokens by depositing ERC-20 tokens (no execution fee)
- **sell-glp**: Redeem GLP tokens for ERC-20 tokens (no execution fee)
- **open-position**: Open a leveraged long or short perpetual position
- **close-position**: Close a perpetual position (partial or full)
- **approve-token**: ERC-20 approval for Router or GlpManager

## Supported Chains

- Arbitrum (chain ID: 42161) — primary
- Avalanche (chain ID: 43114)

## Installation

```bash
npx onchainos plugin install gmx-v1
```

## Usage

```bash
# Check token prices
gmx-v1 get-prices --chain 42161

# Swap 10 USDC to WETH
gmx-v1 swap --chain 42161 \
  --input-token 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --input-amount 10000000 \
  --output-token 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 \
  --dry-run

# Buy GLP with 5 USDC
gmx-v1 buy-glp --chain 42161 \
  --token 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --amount 5000000 \
  --dry-run
```

## License

MIT
