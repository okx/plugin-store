# aave-v3

Aave V3 lending and borrowing plugin for the OnchaionOS Plugin Store.

Lend and borrow crypto assets on Aave V3 — the leading decentralized liquidity protocol — across Ethereum, Polygon, Arbitrum, and Base.

## Supported Chains

| Chain | Chain ID |
|-------|----------|
| Ethereum Mainnet | 1 |
| Polygon | 137 |
| Arbitrum One | 42161 |
| Base | 8453 (default) |

## Operations

| Command | Description |
|---------|-------------|
| `supply` | Deposit assets to earn interest |
| `withdraw` | Redeem aTokens and withdraw underlying |
| `borrow` | Borrow against posted collateral |
| `repay` | Repay borrowed debt (partial or full) |
| `positions` | View current supply and borrow positions |
| `health-factor` | Check account health factor and liquidation risk |
| `reserves` | List market rates, APYs, liquidity for all assets |
| `claim-rewards` | Claim accrued AAVE/GHO/token rewards |
| `set-collateral` | Enable or disable an asset as collateral |
| `set-emode` | Enable efficiency mode for a correlated asset category |

## Usage

```bash
# Supply 1000 USDC on Base (default chain)
aave-v3 supply --asset USDC --amount 1000

# Check health factor on Arbitrum
aave-v3 --chain 42161 health-factor

# Borrow 0.5 ETH on Arbitrum (dry-run first)
aave-v3 --chain 42161 --dry-run borrow --asset WETH --amount 0.5

# Repay all USDC debt on Polygon
aave-v3 --chain 137 repay --asset USDC --all

# List all reserves with APYs
aave-v3 reserves

# View positions
aave-v3 positions
```

## Architecture

- **Supply / Withdraw / Claim Rewards**: Delegated to `onchainos defi invest/withdraw/collect`
- **Borrow / Repay / Set Collateral / Set E-Mode**: Rust binary constructs ABI calldata, submits via `onchainos wallet contract-call`
- **Health Factor / Reserves / Positions**: Rust binary makes `eth_call` via public RPC

## Build

```bash
cargo build --release
```

## License

MIT
