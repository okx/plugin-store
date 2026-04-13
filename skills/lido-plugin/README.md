# Lido Liquid Staking Plugin

Stake ETH with [Lido](https://lido.fi) to receive stETH, request withdrawals, and claim finalized ETH — all via the onchainos Plugin Store.

## Commands

| Command | Description |
|---|---|
| `lido stake` | Stake ETH to receive stETH |
| `lido get-apy` | Get current stETH staking APR |
| `lido balance` | Check stETH balance |
| `lido request-withdrawal` | Request withdrawal of stETH for ETH |
| `lido get-withdrawals` | List pending and past withdrawal requests |
| `lido claim-withdrawal` | Claim finalized withdrawal(s) |

## Requirements

- `onchainos` CLI ≥ 2.0.0
- Wallet logged in for write operations

## Build

```bash
cargo build --release
```

## License

MIT
