---
name: football-match-predictor
description: Use this skill to analyze football/soccer prediction markets on Polymarket, rank match outcome opportunities, build risk-limited trade plans, and execute only confirmed trades through the Polymarket plugin with strategy attribution.
version: "1.0.0"
author: "stefan"
tags:
  - polymarket
  - football
  - soccer
  - prediction-market
  - sports
  - strategy
---

# Football Match Predictor

Analyze football/soccer markets on Polymarket and prepare risk-limited trades.
Use `polymarket-plugin` for all market discovery, orderbook inspection, and
execution. Never place a trade without explicit user confirmation.

This is a prediction-market workflow, not a guarantee of match outcomes. Always
show uncertainty, maximum loss, and the reason a market is being skipped or
included.

## Overview

This skill helps an AI agent find football/soccer event markets on Polymarket,
evaluate market structure, estimate risk-adjusted edge, and prepare trade plans
with strict exposure limits. It is a Skill-only strategy that depends on the
official `polymarket-plugin`; it does not implement its own wallet, signing, or
transaction logic.

## Trigger Phrases

Use this skill when the user asks to:

- predict a football/soccer match
- find football betting or prediction market opportunities
- analyze a Polymarket sports market
- build a football match trade plan
- trade a football match market through Polymarket

## Default Safety Settings

- `strategy_id`: `football-match-predictor`
- default mode: analysis only
- default dry run before live trading: yes
- default max per market: `10 USDC`
- default max session exposure: `30 USDC`
- minimum liquidity: `1000 USDC`
- maximum spread: `0.08`
- minimum time to scheduled event start: `2 hours`
- never use leverage
- never average down automatically
- never trade if market resolution rules are unclear

If the user provides stricter limits, use the stricter limits. If the user asks
for larger live exposure, restate the total maximum loss before requesting final
confirmation.

## Pre-flight Checks

Before live trading:

1. Ensure `polymarket-plugin` is installed and available.
2. Run `polymarket-plugin check-access` and stop if the user is in a restricted
   region.
3. Run `polymarket-plugin balance` and confirm sufficient funds for the planned
   maximum loss.
4. If wallet/session errors occur, follow the recovery guidance from
   `polymarket-plugin` instead of asking for private keys or seed phrases.

Read-only market discovery can run before wallet checks, but no live order can
be prepared for final confirmation until access and balance are checked.

## Workflow

1. Parse the user's target.
   - Teams, league, date, tournament, or general slate.
   - Budget, max per market, risk preference, and live execution preference.
   - If missing, assume analysis-only and `10 USDC` max per market.

2. Discover markets with `polymarket-plugin`.
   - Search for football and soccer terms plus team names.
   - Include common market language such as `winner`, `match`, `qualify`,
     `league`, `cup`, and the league name if provided.
   - Prefer active markets with clear resolution rules.

3. Inspect each candidate.
   - Fetch market details and orderbook.
   - Record outcome prices, bid/ask spread, liquidity, volume, close time, and
     resolution wording.
   - Skip markets with ambiguous wording, low liquidity, stale data, or spreads
     wider than the configured limit.

4. Score candidates.
   - Convert ask price to implied probability.
   - Estimate fair probability only from available evidence. If no team news or
     model evidence is available, label the forecast as market-implied only.
   - Compute edge as `estimated_probability - ask_price`.
   - Apply penalties for wide spread, low liquidity, event timing risk, and
     ambiguous resolution.
   - Rank by risk-adjusted edge, not by excitement or payout.

5. Build the trade plan.
   - Show selected market, outcome, ask price, implied probability, estimated
     probability, edge, liquidity, spread, recommended size, and maximum loss.
   - Use smaller sizing when confidence is low:
     - high confidence: up to 100% of per-market cap
     - medium confidence: up to 50% of per-market cap
     - low confidence: no live trade, watchlist only
   - Total recommended size must not exceed the session cap.

6. Dry run first.
   - Present the exact intended Polymarket action.
   - Use dry-run or preview mode if available in `polymarket-plugin`.
   - Ask for explicit confirmation before any live order.

7. Execute only after confirmation.
   - Use the official Polymarket plugin command or tool for execution.
   - Include `--strategy-id football-match-predictor` on every trade command
     when the interface supports strategy attribution.
   - Reconfirm if price moves, available liquidity changes, or the total maximum
     loss increases.

## Commands

Use these commands through `polymarket-plugin`:

```bash
polymarket-plugin list-markets --keyword "<team-or-football-keyword>"
polymarket-plugin get-market --market-id <slug_or_condition_id>
polymarket-plugin balance
polymarket-plugin buy --market-id <id> --outcome <outcome> --amount <usdc> --price <0-1> --dry-run --strategy-id football-match-predictor
polymarket-plugin buy --market-id <id> --outcome <outcome> --amount <usdc> --price <0-1> --strategy-id football-match-predictor
polymarket-plugin sell --market-id <id> --outcome <outcome> --shares <shares> --price <0-1> --dry-run --strategy-id football-match-predictor
polymarket-plugin sell --market-id <id> --outcome <outcome> --shares <shares> --price <0-1> --strategy-id football-match-predictor
```

Command rules:

- Use `list-markets` first, then `get-market` for each candidate.
- Prefer limit orders with `--price`; use immediate orders only when the user
  explicitly wants immediate execution.
- Always run the `--dry-run` command before the corresponding live command.
- Include `--strategy-id football-match-predictor` for buy and sell commands.
- Display only human-relevant fields from plugin output: market title, outcome,
  price, amount, order id, status, and PnL if available.

## Output Format

Use this format for analysis:

```text
Football Match Predictor

Scope: [teams/league/date]
Mode: [analysis-only | dry-run | live after confirmation]
Limits: max [x] USDC per market, max [y] USDC session exposure

Top markets:
1. [Market title]
   Outcome: [YES/NO/team/outcome]
   Ask: [price] ([implied probability] implied)
   Estimated probability: [estimate or "market-implied only"]
   Edge: [edge or "not estimated"]
   Liquidity: [value]
   Spread: [value]
   Recommendation: [watchlist | dry-run | trade]
   Size: [USDC]
   Max loss: [USDC]
   Reason: [short rationale]

Skipped:
- [Market]: [reason]

Next action:
[No trade / dry-run command / ask for confirmation]
```

For live trading, ask a direct confirmation question:

```text
Confirm live Polymarket trade:
[buy/sell] [outcome] on [market] for up to [amount] USDC.
Maximum loss: [amount] USDC.
Strategy id: football-match-predictor.

Reply "confirm" to execute.
```

## Polymarket Execution Rules

- Route all market access and order execution through `polymarket-plugin`.
- Do not construct raw Polymarket transactions manually.
- Do not request, store, or print private keys or seed phrases.
- Do not bypass wallet confirmation.
- Do not split orders to avoid limits.
- Do not trade resolved, paused, unclear, or low-liquidity markets.
- If a trade is executed, report filled amount, average price, market, outcome,
  transaction/order id if available, and remaining session exposure.

## Error Handling

| Error | Cause | Resolution |
|-------|-------|------------|
| Restricted access | Region is not supported | Stop before funding or trading. |
| Market not found | Team/league/date query is ambiguous | Ask for the exact team, league, date, or Polymarket URL. |
| Low liquidity | Orderbook cannot support the requested size | Reduce size or skip the market. |
| Wide spread | Execution price is unreliable | Skip unless the user explicitly accepts the spread. |
| Insufficient balance | Wallet/proxy lacks funds | Lower exposure or ask the user to fund through the official plugin flow. |
| Session expired | Onchain OS authentication expired | Follow `polymarket-plugin` session recovery; never request keys. |
| Price moved after dry-run | Market changed before confirmation | Re-run analysis and ask for confirmation again. |

## Forecasting Rules

Use cautious language:

- Prefer `market-implied probability`, `estimated probability`, and
  `risk-adjusted edge`.
- Avoid absolute claims such as `lock`, `guaranteed`, or `sure win`.
- If current team news, lineups, injuries, or weather are unavailable, say so and
  avoid creating a fake precision forecast.
- If the user supplies team context, separate user-provided assumptions from
  market data.

When only market data is available, the skill may still rank markets by
liquidity, spread, implied probability, and risk controls, but it must label the
result as market-structure analysis rather than a fundamental match prediction.

## Refusal and Pause Conditions

Pause and ask for clarification instead of trading when:

- The requested market cannot be clearly matched.
- Resolution wording is ambiguous.
- The event appears already started and the user did not explicitly request live
  in-play analysis.
- The user asks to exceed their configured budget or the default session cap.
- The user asks for guaranteed profit or asks to hide risk.

Refuse to continue if the user asks for private-key handling, evasion of wallet
confirmation, market manipulation, or misleading promotion of results.

## Security Notices

- This skill can lead to real-money Polymarket trades only after explicit user
  confirmation.
- Prediction markets can resolve to zero. Show maximum loss for every proposed
  trade.
- Do not provide gambling-style guarantees or hide uncertainty.
- Do not handle private keys, seed phrases, raw signatures, or manual
  transaction construction.
- Do not call undeclared external APIs from this skill. All Polymarket data and
  execution must route through `polymarket-plugin`.
