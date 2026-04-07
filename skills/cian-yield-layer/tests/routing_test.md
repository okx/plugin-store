# Skill Routing Test — cian-yield-layer

**Source:** `skills/cian-yield-layer/SKILL.md`
**Test Date:** 2026-04-05

---

## Routing Rules (from SKILL.md)

The skill triggers when user mentions any of:
- CIAN, CIAN Yield Layer, ylstETH, ylpumpBTC
- "deposit stETH CIAN", "stake ETH CIAN yield", "CIAN recursive staking"
- "CIAN withdraw", "request redeem CIAN", "CIAN APY", "CIAN TVL"
- "CIAN yield layer position", "ylstETH balance", "ylpumpBTC balance"
- "pumpBTC yield", "CIAN pumpBTC vault", "CIAN stETH vault"

**Do NOT trigger for:**
- General ETH staking unrelated to CIAN
- CIAN products other than Yield Layer

---

## Positive Cases (SHOULD route to cian-yield-layer)

| # | User Input | Match Keyword | Expected |
|---|-----------|--------------|---------|
| P-01 | "What are the CIAN vaults?" | "CIAN" / "CIAN vaults" | ROUTE |
| P-02 | "Show me ylstETH APY" | "ylstETH", "CIAN APY" | ROUTE |
| P-03 | "I want to deposit stETH CIAN" | "deposit stETH CIAN" | ROUTE |
| P-04 | "What is the CIAN TVL?" | "CIAN TVL" | ROUTE |
| P-05 | "Check my ylpumpBTC balance" | "ylpumpBTC balance" | ROUTE |
| P-06 | "CIAN withdraw my stETH" | "CIAN withdraw" | ROUTE |
| P-07 | "request redeem CIAN ylstETH" | "request redeem CIAN" | ROUTE |
| P-08 | "What is my CIAN yield layer position?" | "CIAN yield layer position" | ROUTE |
| P-09 | "stake ETH CIAN yield" | "stake ETH CIAN yield" | ROUTE |
| P-10 | "How much pumpBTC yield from CIAN?" | "pumpBTC yield" / "CIAN" | ROUTE |
| P-11 | "CIAN recursive staking explained" | "CIAN recursive staking" | ROUTE |
| P-12 | "redeem ylstETH" | "ylstETH" | ROUTE |

---

## Negative Cases (SHOULD NOT route to cian-yield-layer)

| # | User Input | Reason | Expected |
|---|-----------|--------|---------|
| N-01 | "Stake ETH on Lido" | General staking, no CIAN mention | SKIP |
| N-02 | "swap ETH for USDC on Uniswap" | Unrelated DApp | SKIP |
| N-03 | "What is my stETH balance on Aave?" | Different protocol | SKIP |
| N-04 | "Buy pumpBTC on OKX" | Purchase action, no CIAN yield mention | SKIP |
| N-05 | "Deposit WBTC into Compound" | Different protocol | SKIP |

---

## Skill Metadata Analysis

```yaml
name: cian-yield-layer
description: >-
  Use when the user asks about CIAN Yield Layer, CIAN vaults, ylstETH, ylpumpBTC,
  'deposit stETH CIAN', 'stake ETH CIAN yield', 'CIAN recursive staking', ...
```

**Assessment:** Description is specific enough to distinguish from general ETH staking.
The "Do NOT use" guards are correct.

**Coverage:** All 5 commands (vaults, balance, positions, deposit, request-redeem) are documented.

**Result: PASS** — Routing rules are accurate and appropriately scoped.
