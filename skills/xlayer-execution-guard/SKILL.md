---
name: xlayer-execution-guard
description: "Complete pre-execution judgment + post-execution proof pipeline for autonomous AI agents with closed-loop validation"
version: 1.0.0
---

# X Layer Execution Guard

A combined pre-execution judgment + post-execution proof pipeline for autonomous AI agents on X Layer with closed-loop validation.

## What It Does

- **Pre-execution**: Judge routes BEFORE trade → Execute / Resize / Retry / Block verdict
- **Post-execution**: Generate standardized proof AFTER execution with outcome attribution
- **Closed-loop**: Compare verdict with outcome to help agents LEARN

## When to Use This Skill

Use this skill when an agent needs:

- Route quality judgment BEFORE executing a trade
- Honeypot detection (isHoneyPot)
- Price impact analysis
- Execution proof with outcome attribution AFTER trading
- Complete audit trail from intent to receipt

## Installation

```bash
npx skills add okx/plugin-store --skill xlayer-execution-guard
```

Or in Python:

```bash
pip install xlayer-execution-guard
```

## Quick Start

```python
import asyncio
from execution_guard import ExecutionGuard, GuardIntent

async def main():
    guard = ExecutionGuard()

    intent = GuardIntent(
        agent_name="my_agent",
        intent_id="trade-001",
        from_token="USDC",
        to_token="USDT",
        amount="100",
        slippage_percent="0.5",
        reason="swap USDC for USDT",
        execute_after_verdict=True
    )

    result = await guard.run(intent)
    print(result.agent_summary)

asyncio.run(main())
```

## API Integration

Built with REAL OnchainOS DEX Aggregator APIs:

| API | Purpose |
|-----|---------|
| `/api/v6/dex/aggregator/all-tokens` | Token discovery |
| `/api/v6/dex/aggregator/get-liquidity` | Liquidity sources |
| `/api/v6/dex/aggregator/quote` | Route quoting (aggregated + 4 DEXes) |

## Pre-execution Verdict

| Verdict | Meaning |
|---------|---------|
| `execute` | Route acceptable, proceed with trade |
| `resize` | Acceptable but reduce trade size |
| `retry` | Not safe now, try again later |
| `block` | Should not execute |

## Post-execution Attribution

| Attribution | Meaning |
|-------------|---------|
| `success` | Trade completed as intended |
| `thesis_error` | Strategy was fundamentally wrong |
| `timing_error` | Strategy right but timing off |
| `execution_error` | Execution layer failed |

## Safety Checks

Every trade runs through 13 automated checks:

1. `quote_available` — Is output available?
2. `price_impact` — Is impact within limits?
3. `fallback_coverage` — Are there fallback DEXes?
4. `banned_dex_exclusion` — No banned DEXes in route?
5. `agent_reason` — Does caller have a reason?
6. `token_safety` — Honeypot detection
7. `gas_estimate` — Gas available?
8. `trade_fee` — Fees reasonable?
9. `from_token_price` — Token price available?
10. `to_token_price` — Token price available?
11. `from_token_tax` — No hidden tax?
12. `to_token_tax` — No hidden tax?
13. `dex_comparison` — Multi-DEX check done?

## Multi-DEX Comparison

Compares quotes across:

- Uniswap V3
- QuickSwap V3
- CurveNG
- DackieSwap V3

## Closed-Loop Validation

The innovation: Compare pre-execution verdict with post-execution outcome.

```
verdict="execute" + outcome="success" → ✅ CORRECT
verdict="execute" + outcome="failed"  → attribution=execution_error
verdict="block" + outcome="failed"   → ✅ CORRECT (blocked risky trade)
verdict="block" + outcome="success"   → ⚠️ possible false positive
```

This feedback loop helps agents LEARN from past decisions.

## Output Structure

```json
{
  "verdict": "execute",
  "pre_execution": {
    "recommended_route": "QuickSwap V3 → CurveNG",
    "output_amount": "98 USDT",
    "price_impact": "1.95%",
    "risk_level": "medium",
    "checks": [...]
  },
  "execution": {
    "tx_hash": "0x...",
    "status": "success"
  },
  "post_execution": {
    "proof_id": "guard_xxx",
    "outcome": "success",
    "attribution": "execution_completed"
  },
  "closed_loop_validation": {
    "verdict_match": true,
    "analysis": "Pre-execution verdict correctly predicted success"
  }
}
```

## Live Proof

Captured April 15, 2026 with REAL OnchainOS API.

- 13 checks passed
- 2 alternative routes compared
- Closed-loop validation verified
- Test pair: USDC → USDT → Verdict: execute, Risk: medium

## License

MIT