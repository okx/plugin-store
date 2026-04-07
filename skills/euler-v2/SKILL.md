# euler-v2 Skill

Interact with **Euler V2** modular lending vaults (EVaults) — ERC-4626-compatible vaults with borrowing functionality, connected via the Ethereum Vault Connector (EVC).

## Available Commands

### markets
List available Euler V2 lending markets on a chain.

```
euler-v2 [--chain <id>] markets [--asset <SYMBOL>]
```

**Examples:**
- `euler-v2 --chain 8453 markets` — list all Base markets
- `euler-v2 --chain 8453 markets --asset USDC` — filter for USDC vaults
- `euler-v2 --chain 1 markets` — list Ethereum mainnet markets

---

### positions
View your current supply and borrow positions.

```
euler-v2 [--chain <id>] [--dry-run] positions
```

---

### supply
Deposit underlying assets into an Euler V2 EVault.

> **Ask user to confirm** before executing: display vault address, asset, amount, chain.

```
euler-v2 [--chain <id>] [--dry-run] supply --vault <VAULT> --amount <N>
```

**`--vault`**: vault address (`0x...`) or known symbol (`USDC`, `WETH`, `CBBTC`)
**`--amount`**: human-readable amount (e.g. `10` or `0.001`)

**Examples:**
- `euler-v2 --chain 8453 supply --vault USDC --amount 10` — supply 10 USDC on Base
- `euler-v2 --chain 8453 --dry-run supply --vault 0x0a1a3b5f2041f33522c4efc754a7d096f880ee16 --amount 5`

---

### withdraw
Withdraw underlying assets from an Euler V2 EVault.

> **Ask user to confirm** before executing.

```
euler-v2 [--chain <id>] [--dry-run] withdraw --vault <VAULT> [--amount <N>] [--all]
```

**Examples:**
- `euler-v2 --chain 8453 withdraw --vault USDC --amount 5`
- `euler-v2 --chain 8453 withdraw --vault USDC --all`

---

### borrow
Simulate borrowing from an Euler V2 EVault (**dry-run only**).

> Borrowing is **dry-run only** — liquidation risk requires careful collateral management via EVC.

```
euler-v2 --dry-run [--chain <id>] borrow --vault <VAULT> --amount <N>
```

---

### repay
Simulate repaying debt in an Euler V2 EVault (**dry-run only**).

```
euler-v2 --dry-run [--chain <id>] repay --vault <VAULT> [--amount <N>] [--all]
```

---

## Supported Chains

| Chain     | ID    |
|-----------|-------|
| Base      | 8453  |
| Ethereum  | 1     |
| Arbitrum  | 42161 |
| Avalanche | 43114 |
| BSC       | 56    |

## Known Vault Symbols (Base 8453)

| Symbol | Vault Address                               | Underlying |
|--------|---------------------------------------------|------------|
| USDC   | 0x0a1a3b5f2041f33522c4efc754a7d096f880ee16  | USDC       |
| WETH   | 0x859160db5841e5cfb8d3f144c6b3381a85a4b410  | WETH       |
| CBBTC  | 0x7b181d6509deabfbd1a23af1e65fd46e89572609  | cbBTC      |

## Notes

- `borrow` and `repay` are always dry-run only.
- Real borrowing requires enabling collateral and controller via EVC first.
- Use `markets` to discover vault addresses on other chains.
