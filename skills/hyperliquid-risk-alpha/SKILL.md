---
name: hyperliquid-risk-alpha
description: "Use Hyperliquid Risk Alpha when a user wants a small, risk-controlled BTC or ETH Hyperliquid trade workflow with readiness checks, opportunity scoring, confirmation, protection, and lifecycle review."
version: "1.0.0"
author: "cbl980712-coder"
tags:
  - hyperliquid
  - perpetuals
  - trading
  - risk-management
  - onchainos
---

# Hyperliquid Risk Alpha

Hyperliquid Risk Alpha is a small-size, risk-first trading skill built on the official Hyperliquid Plugin and Onchain OS.

It helps users answer:

> "Am I ready to trade, is there a reasonable BTC or ETH opportunity, and how do I avoid entering an unmanaged position?"

It does not promise profit or high win rate. It focuses on readiness, execution quality, position protection, and lifecycle review.

## What It Does

Use this skill to:

- Check account, funding, and trading readiness.
- Detect whether funds are in the correct trading layer.
- Read BTC and ETH market data.
- Score fresh opportunities as:
  - no_trade
  - watch
  - tradable_light
  - tradable_full
- Build a small order plan.
- Run risk check and dry-run before any live order.
- Require final user confirmation before live trading.
- Submit a small live trade only after confirmation.
- Run post-entry guard.
- Build an executable exit plan.
- Prefer reduce-only protection.
- Track trade lifecycle.
- Update trade journal and guard state.
- Produce a final review after exit.

## Product Rules

- Strategy ID: `risk-alpha-xy`
- Underlying official plugin: `hyperliquid-plugin`
- First version markets:
  - BTC
  - ETH
- Default style:
  - small size
  - low leverage
  - single position
  - no automatic add-on
  - no continuous trading loop
- Readiness ready means trading is possible, not that trading is required.
- No live order without explicit user confirmation.

## Public Command Assumptions

The user should have the official plugin installed and available on PATH:

```bash
hyperliquid-plugin
onchainos
```

Do not use project-local wrappers, copied binaries, relays, or diagnostic tools for formal execution.

## Normal User Flow

1. Readiness:
   - account readiness
   - funding readiness
   - trading readiness

2. Market data:
   - candles
   - ATR
   - funding
   - spread
   - basic orderbook depth
   - tiny price impact

3. Opportunity score:
   - no_trade
   - watch
   - tradable_light
   - tradable_full

4. Order plan:
   - symbol
   - direction
   - size
   - maximum acceptable price
   - leverage
   - risk boundary

5. Risk and dry-run:
   - platform minimum size
   - balance
   - leverage
   - spread
   - liquidity
   - execution path

6. Final confirmation:
   - no live order without explicit confirmation

7. Post-entry management:
   - check whether a position exists
   - detect naked position risk
   - build executable exit plan
   - prefer reduce-only protection
   - update journal/state

8. Final review:
   - entry
   - exit
   - PnL
   - exit attribution
   - whether the trade followed plan

## Output Style

Write for a normal user, not for a developer.

Preferred one-screen answer:

- Is the account ready?
- Is funding ready?
- Is trading ready?
- Is there an open position?
- Is the position protected?
- Is a new trade allowed?
- Which of BTC/ETH is better, if any?
- If no trade, why?
- If trade is possible, what is the plan and exit?

## Safety Boundaries

- Do not promise profit.
- Do not claim high win rate.
- Do not open a live trade without final confirmation.
- Do not open a second trade if an existing position is unmanaged.
- Do not hide insufficient market data.
- Do not expose private wallet data, local machine paths, API keys, sessions, or logs to the user.
- If a trade is too small for meaningful exit management, use a simpler full-position protection plan instead of fake multi-target exits.

## Error Handling

| Condition | Meaning | Response |
|---|---|---|
| account_ready is false | Account is not ready | Stop and explain the one required next action |
| funding_ready is false | Funds are not in the usable trading layer | Stop before order planning |
| trading_ready is false | Trading path is not ready | Stop before order planning |
| weak opportunity | Setup is not worth forcing | Output watch or no_trade |
| open position unprotected | Risk is unmanaged | Prioritize post-entry guard and protection |
| dry-run fails | Execution path is not safe | Do not request final confirmation |
| user does not confirm | No live order permission | Stop without submitting |

## Example User Request

> "Use Hyperliquid Risk Alpha to see if BTC or ETH is worth a small trade today."

Good response:

- "Account and funding are ready."
- "ETH has better structure, but this is only tradable_light."
- "Here is the small order plan and exit."
- "Run risk check and dry-run first."
- "No live order will be submitted until you confirm."
