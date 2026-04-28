---
name: polymarket-event-alpha
description: "Use Polymarket Event Alpha when a user wants help finding, screening, planning, and risk-checking Polymarket event trades without forcing poor markets or unsafe execution."
version: "1.0.0"
author: "cbl980712-coder"
tags:
  - polymarket
  - prediction-market
  - event-trading
  - risk-management
  - onchainos
---

# Polymarket Event Alpha

Polymarket Event Alpha is a user-facing event market trading skill built on the official Polymarket Plugin and Onchain OS.

It helps users answer a simple question:

> "Can I safely consider an event trade right now, and if yes, which market is actually worth looking at?"

It does not claim guaranteed outcomes, high win rate, or automatic profit. Its value is execution quality: fewer bad markets, fewer bad spreads, fewer wrong-asset mistakes, clearer risk checks, and a concrete exit plan before any confirmed trade.

## What It Does

Use this skill to:

- Check whether the user is ready to trade Polymarket.
- Identify whether the required asset is available.
- Detect missing USDC.e, gas, or setup requirements.
- Rank event markets with a three-part score:
  - market quality
  - directional edge
  - execution feasibility
- Produce a Top 3 event-market shortlist.
- Explain the Top 1 market in plain language.
- Build a proposed trade plan with budget, side, price boundary, and risk notes.
- Build an exit playbook with TP1, TP2, invalidation exit, and time exit.
- Stop invalid live attempts when platform access conditions are not satisfied.

## Product Rules

- Strategy ID: `event-alpha-xy`
- Underlying official plugin: `polymarket-plugin`
- Supported first version market pools:
  - politics
  - macro
  - crypto event
- Sports are excluded in v1.
- The skill must not force a YES/NO direction when directional edge is weak.
- The skill must not default to automatic swap.
- The skill must not default to automatic betting.
- Live execution is only allowed when platform access, account readiness, asset readiness, risk checks, and user confirmation all pass.

## Public Command Assumptions

The user should have the official plugin installed and available on PATH:

```bash
polymarket-plugin
onchainos
```

Do not use project-local wrappers, copied binaries, relays, or diagnostic tools for formal execution.

## Normal User Flow

1. Readiness check:
   - account status
   - required asset
   - gas/setup condition
   - platform access condition

2. Asset preparation advice:
   - explain what is missing
   - prefer direct USDC.e preparation when needed
   - only discuss swap as a confirmed, separately reviewed action

3. Market shortlist:
   - rank markets by quality and execution feasibility
   - return Top 3
   - explain why some markets should only be watched

4. Trade plan:
   - proposed side only when directional edge is strong enough
   - budget
   - entry price boundary
   - maximum acceptable execution price
   - risk notes

5. Exit playbook:
   - TP1
   - TP2
   - invalidation exit
   - time exit

6. Confirmation:
   - no live trade without explicit user confirmation

## Output Style

Write for a normal user, not for a developer.

Preferred one-screen answer:

- Can I trade now?
- What is missing, if anything?
- What are the Top 3 markets?
- Which one is most worth watching?
- Why not force a trade?
- If trading is allowed, what is the plan?
- How should the user exit?

## Safety Boundaries

- Do not promise profit.
- Do not claim a market is certain.
- Do not bypass access restrictions.
- Do not trade if assets, setup, or platform access are not ready.
- Do not hide bad spread, thin orderbook, or weak directional edge.
- Do not expose private wallet data, local machine paths, API keys, sessions, or logs to the user.

## Error Handling

| Condition | Meaning | Response |
|---|---|---|
| Missing required asset | User lacks usable USDC.e or gas | Explain the missing asset and safest preparation path |
| Weak directional edge | Market may be interesting but not actionable | Output watch-only or price-target-only |
| Wide spread or thin book | Execution quality is poor | Reject or keep on watchlist |
| Access restricted | Live trading is not available | Keep readiness, shortlist, plan, risk, and dry-run as product outputs; do not push live |
| Incomplete market data | Not enough evidence | Say insufficient data and avoid a forced direction |

## Example User Request

> "Find me the best small Polymarket event trade today."

Good response:

- "You are not ready because USDC.e is missing."
- or "You are ready, but the best markets are watch-only because directional edge is weak."
- or "This market is a small-trade candidate. Here is the budget, side, entry price, risk limit, and exit playbook. Confirm before execution."
