# sparklend — SparkLend Lending & Borrowing on Ethereum

## Commands

| Command | Description | Trigger phrases |
|---------|-------------|-----------------|
| `sparklend supply` | Supply an asset to earn interest (spTokens minted) | "supply", "deposit", "provide liquidity", "lend" |
| `sparklend withdraw` | Withdraw a previously supplied asset | "withdraw", "remove liquidity", "get back my tokens" |
| `sparklend borrow` | Borrow against posted collateral at variable rate | "borrow", "take out a loan", "leverage" |
| `sparklend repay` | Repay outstanding borrow (full or partial) | "repay", "pay back", "close position", "pay off debt" |
| `sparklend positions` | View current supply/borrow positions | "my positions", "what have I supplied", "what do I owe" |
| `sparklend health-factor` | Check health factor and liquidation risk | "health factor", "am I safe", "liquidation risk" |
| `sparklend reserves` | List market rates and APYs for all assets | "rates", "APY", "interest rates", "what can I supply" |

## Trigger Phrases

- "use SparkLend" → run `sparklend positions` to check current state, then guide user
- "supply DAI to SparkLend" → `sparklend supply --asset DAI --amount <X>`
- "borrow USDC on SparkLend" → `sparklend borrow --asset USDC --amount <X>`
- "what's my health factor" → `sparklend health-factor`
- "repay my SparkLend loan" → `sparklend repay --asset <ASSET> --all`
