# StakeStone Plugin

Stake ETH with StakeStone liquid staking protocol to receive STONE yield-bearing tokens on Ethereum mainnet.

## Features

- **Stake ETH** - Deposit ETH into StakeStone vault, receive STONE tokens
- **Request Withdrawal** - Queue STONE for withdrawal back to ETH
- **Cancel Withdrawal** - Cancel a pending withdrawal request
- **Get Rate** - View current STONE/ETH exchange rate and vault TVL
- **Get Position** - Check your STONE balance and pending withdrawals

## Usage

```bash
# Stake 0.001 ETH
stakestone stake --amount 0.001

# Check exchange rate
stakestone get-rate

# Check your position
stakestone get-position

# Queue 0.001 STONE for withdrawal
stakestone request-withdraw --amount 0.001

# Cancel withdrawal
stakestone cancel-withdraw --amount 0.001

# Dry run (no broadcast)
stakestone stake --amount 0.001 --dry-run
```

## Contracts (Ethereum Mainnet)

| Contract | Address |
|---|---|
| StoneVault | `0xA62F9C5af106FeEE069F38dE51098D9d81B90572` |
| STONE token | `0x7122985656e38BDC0302Db86685bb972b145bD3C` |

## License

MIT
