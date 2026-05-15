# xlayer-bridge-yield

Bridge assets to X Layer (OKX's L2) and auto-optimize stablecoin yield farming with built-in risk management.

## What It Does

This OKX OnchainOS plugin provides a seamless "bridge and farm" experience:

1. **Bridge Optimizer** — Compares cross-chain bridge routes to find the cheapest and fastest path to X Layer
2. **Yield Scanner** — Scans DeFi protocols on X Layer and ranks yield pools by APY, TVL, and security risk
3. **Full Pipeline** — One command to bridge + deposit into the best pool, with pre-execution safety checks
4. **Position Monitor** — Tracks active positions with depeg alerts, stop-loss triggers, and rebalance suggestions

## Key Features

- **Dry-run by default** — No real transactions unless you explicitly confirm
- **Security-first** — Mandatory token scans before any deposit
- **Risk management** — Configurable stop-loss, max amounts, and depeg alerts
- **X Layer optimized** — Takes advantage of near-zero gas for frequent rebalancing

## Install

```bash
npx skills add okx/plugin-store --skill xlayer-bridge-yield
```

## Usage Examples

```
"Find the cheapest bridge for 100 USDC from Ethereum to X Layer"
"Show best yield pools on X Layer for USDT"
"Bridge 200 USDC to X Layer and farm the best yield"
"Check my X Layer farming positions"
```

## Requirements

- onchainos CLI installed (`npx skills add okx/onchainos-skills`)
- Python 3.8+
- Funded wallet on source chain

## Safety

- Default dry-run mode
- Pre-trade security scanning
- Configurable stop-loss (-5% default)
- Max transaction amount ($1,000 default)
- Stablecoin depeg alerts (0.5% threshold)

## License

MIT
