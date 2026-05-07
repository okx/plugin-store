# kamino-liquidity

Kamino Liquidity KVault earn vaults on Solana.

## Commands

- `vaults` — List all available KVault earn vaults
- `positions` — View your share balances across all vaults
- `deposit` — Deposit tokens into a vault to earn yield
- `withdraw` — Redeem shares for underlying tokens

## Usage

```bash
kamino-liquidity vaults --chain 501
kamino-liquidity positions --chain 501
kamino-liquidity deposit --vault <VAULT_ADDRESS> --amount 0.001 --chain 501
kamino-liquidity withdraw --vault <VAULT_ADDRESS> --amount 1 --chain 501
```

## Chain Support

Solana mainnet only (chain 501).
