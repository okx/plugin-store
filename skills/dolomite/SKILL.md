# dolomite Skill

Interact with **Dolomite** isolated lending markets — supply assets to earn yield, view positions, and simulate borrowing/repayment.

Dolomite uses a central `DolomiteMargin` vault contract. All operations are routed through `operate()` with typed actions (Deposit/Withdraw).

## Available Commands

### markets
List all available Dolomite lending markets.

```
dolomite [--chain <id>] markets [--asset <SYMBOL>]
```

**Examples:**
- `dolomite --chain 42161 markets` — list all Arbitrum markets
- `dolomite --chain 42161 markets --asset USDC` — filter for USDC market

---

### positions
View your current Dolomite supply and borrow positions.

```
dolomite [--chain <id>] [--dry-run] positions
```

**Examples:**
- `dolomite --chain 42161 positions`
- `dolomite --chain 42161 --dry-run positions`

---

### deposit
Supply tokens to Dolomite to earn lending yield.

> **Ask user to confirm** before executing: display asset, amount, chain, and DolomiteMargin address.

```
dolomite [--chain <id>] [--dry-run] deposit --asset <ASSET> --amount <N>
```

**`--asset`**: token symbol (`USDC`, `WETH`, `USDT`) or contract address (`0x...`)
**`--amount`**: human-readable amount (e.g. `10` or `0.001`)

**Examples:**
- `dolomite --chain 42161 deposit --asset USDC --amount 10`
- `dolomite --chain 42161 --dry-run deposit --asset WETH --amount 0.001`

**Note:** Requires two transactions: ERC-20 approve + DolomiteMargin.operate().

---

### withdraw
Withdraw supplied tokens from Dolomite.

> **Ask user to confirm** before executing.

```
dolomite [--chain <id>] [--dry-run] withdraw --asset <ASSET> [--amount <N>] [--all]
```

**Examples:**
- `dolomite --chain 42161 withdraw --asset USDC --amount 5`
- `dolomite --chain 42161 withdraw --asset USDC --all`

---

### borrow
Simulate borrowing tokens from Dolomite (**dry-run only**).

> Borrowing is **always dry-run only** — liquidation risk requires careful collateral management.
> Ensure you have sufficient collateral supplied in other markets.

```
dolomite --dry-run [--chain <id>] borrow --asset <ASSET> --amount <N>
```

**Examples:**
- `dolomite --chain 42161 --dry-run borrow --asset USDC --amount 100`

---

### repay
Simulate repaying debt in Dolomite (**dry-run only**).

> Repay is always dry-run only. To repay on-chain, use the `deposit` command with the borrowed asset.

```
dolomite --dry-run [--chain <id>] repay --asset <ASSET> [--amount <N>] [--all]
```

**Examples:**
- `dolomite --chain 42161 --dry-run repay --asset USDC --amount 100`
- `dolomite --chain 42161 --dry-run repay --asset USDC --all`

---

## Supported Chains

| Chain      | ID    |
|------------|-------|
| Arbitrum   | 42161 |
| Mantle     | 5000  |
| Berachain  | 80094 |

## Known Asset Symbols (Arbitrum 42161)

| Symbol | Token Address                                  | Decimals |
|--------|------------------------------------------------|----------|
| USDC   | 0xaf88d065e77c8cC2239327C5EDb3A432268e5831     | 6        |
| WETH   | 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1     | 18       |
| USDT   | 0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9     | 6        |
| WBTC   | 0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f     | 8        |
| ARB    | 0x912CE59144191C1204E64559FE8253a0e49E6548     | 18       |

## Notes

- `borrow` and `repay` are always dry-run only.
- Deposit requires ERC-20 approve + operate() — two separate transactions.
- Use `markets` to discover available assets on each chain.
- Default chain is Arbitrum (42161).
