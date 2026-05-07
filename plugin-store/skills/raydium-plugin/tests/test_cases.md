# Test Cases — Raydium AMM Plugin

## L1: Compile & Help

| # | Command | Expected |
|---|---------|----------|
| 1.1 | `raydium --help` | Shows usage with all 6 subcommands listed |
| 1.2 | `raydium swap --help` | Shows swap arguments including `--input-mint`, `--output-mint`, `--amount` |
| 1.3 | `raydium get-swap-quote --help` | Shows quote arguments |

---

## L2: Read Operations (live API calls)

### get-token-price

| # | Command | Expected |
|---|---------|----------|
| 2.1 | `raydium get-token-price --mints So11111111111111111111111111111111111111112` | `success: true`, SOL price ~$50–$200 USD |
| 2.2 | `raydium get-token-price --mints EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | USDC price ~$1.00 |
| 2.3 | `raydium get-token-price --mints So11111111111111111111111111111111111111112,EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | Both prices returned |

### get-swap-quote

| # | Command | Expected |
|---|---------|----------|
| 2.4 | `raydium get-swap-quote --input-mint So11111111111111111111111111111111111111112 --output-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1000000000 --slippage-bps 50` | `success: true`, `data.outputAmount` ~79000000, `data.routePlan` non-empty |
| 2.5 | `raydium get-swap-quote --input-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --output-mint So11111111111111111111111111111111111111112 --amount 100000000 --slippage-bps 100` | USDC→SOL quote returned |

### get-price

| # | Command | Expected |
|---|---------|----------|
| 2.6 | `raydium get-price --input-mint So11111111111111111111111111111111111111112 --output-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1000000000` | `price` field ~79 (SOL/USDC ratio), `priceImpactPct` field present |

### get-pools

| # | Command | Expected |
|---|---------|----------|
| 2.7 | `raydium get-pools --mint1 So11111111111111111111111111111111111111112 --mint2 EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --pool-type all --sort-field liquidity` | `success: true`, pools returned with `id`, `type`, `tvl`, `feeRate` |
| 2.8 | `raydium get-pools --ids 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2` | Pool info by ID |
| 2.9 | `raydium get-pools` (no --ids or --mint1) | Error: "Either --ids or --mint1 must be provided" |

### get-pool-list

| # | Command | Expected |
|---|---------|----------|
| 2.10 | `raydium get-pool-list --pool-type all --sort-field liquidity --sort-type desc --page-size 5 --page 1` | `success: true`, `data.data` array of 5 pools, `data.hasNextPage` present |
| 2.11 | `raydium get-pool-list --pool-type concentrated --sort-field apr24h --sort-type desc --page-size 3 --page 1` | CLMM pools returned |

---

## L3: Write Operation — Dry Run

| # | Command | Expected |
|---|---------|----------|
| 3.1 | `raydium --dry-run swap --input-mint So11111111111111111111111111111111111111112 --output-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 1000000000 --slippage-bps 50` | `dry_run: true`, no onchainos call made, no wallet resolution attempted |

---

## L4: Write Operation — Live Swap (requires onchainos login)

> Prerequisites: `onchainos` logged in on Solana mainnet, wallet has SOL balance.

| # | Command | Expected |
|---|---------|----------|
| 4.1 | `raydium swap --input-mint So11111111111111111111111111111111111111112 --output-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --amount 10000000 --slippage-bps 100` | `ok: true`, `transactions[0].txHash` non-empty Solana signature, verify on solscan.io |

---

## Error Cases

| # | Command | Expected |
|---|---------|----------|
| E1 | `raydium swap --input-mint INVALID --output-mint INVALID --amount 100` | API error response or anyhow bail |
| E2 | `raydium get-pools` (missing required args) | Clap error: missing `--ids` or `--mint1` |
| E3 | `raydium swap` (missing --input-mint) | Clap error: required argument missing |
