---
name: safe-competition-trading-agent
description: "Competition-aware Agentic Wallet trading skill that uses onchainOS data, risk firewall checks, and explicit user confirmation before execution"
version: "1.0.0"
author: "Safe Competition Trading Agent Contributors"
tags:
  - trading
  - competition
  - agentic-wallet
  - xlayer
  - solana
  - security
  - risk
---

# Safe Competition Trading Agent

## Overview

Use this skill when the user wants an Agentic Wallet agent to plan or execute a safer competition trade. This is the main judged strategy skill: it uses onchainOS as the primary data source and trading tool, then uses `agent-workflow-composer` and `agent-risk-firewall` as safety layers.

The skill is competition-aware. It checks whether the wallet is registered, whether the competition is active, which chains count, whether the candidate is a real token trade, and whether the quote/unsigned transaction passes a risk gate before the agent asks the user to confirm execution.

Default mode is `dry-run`. Live execution is allowed only after:

1. onchainOS competition detail and user-status are known.
2. onchainOS quote and unsigned transaction context are prepared.
3. `agent-risk-firewall check` returns `allow`, or returns `warn` and the user explicitly confirms.
4. The user has explicitly requested execution.

## Trigger Phrases

Use this skill for requests like:

- "trade for competition"
- "safe competition trade"
- "agentic trading competition"
- "find a safe token for competition"
- "trade 10 dollars for contest"
- "optimize my competition rank"
- "increase my competition volume"
- "dry-run competition trade"
- "run a safe competition swap"
- "join competition and trade"
- "giao dich cho cuoc thi"
- "trade cho competition"
- "tim token an toan de thi"
- "toi uu thu hang cuoc thi"
- "tang volume cuoc thi"
- "giao dich 10 do cho competition"
- "kiem tra trade nay co tinh diem khong"

Do not use this skill for generic wallet balance, normal token portfolio checks, exporting wallets, or unrelated DeFi positions. Use the corresponding wallet, portfolio, DeFi, or DEX skills for those.

## Required onchainOS Role

This skill must use onchainOS as the primary source of truth:

| Purpose | Required command family |
|---|---|
| Wallet session | `onchainos wallet status` |
| Competition discovery and rules | `onchainos competition list`, `onchainos competition detail` |
| Registration status | `onchainos competition user-status` |
| Token discovery and metadata | `onchainos token ...` or the active onchainOS token skill |
| Quote | `onchainos swap quote` |
| Unsigned transaction context | onchainOS swap transaction preparation command; prepare only, do not sign or broadcast |
| Live execution after gates | `onchainos swap execute` |

Use `agent-workflow-composer` to validate workflow order and `agent-risk-firewall` to produce the final risk verdict.

## Competition Invariants

Apply these rules when building the plan:

- Every active competition currently counts Solana plus the backend primary chain. If detail says X Layer, treat both X Layer and Solana as supported until backend exposes a full multi-chain field.
- Trades on either supported chain count toward the same standing.
- Keep internal ids such as `activityId`, `chainIndex`, and `accountId` in tool context only. Do not render them in user-visible messages.
- Identify competitions to the user by name, not by numeric id.
- `myRankInfo.userTotal = 0` can mean the user has not hit the threshold or the backend has not updated yet; it does not mean the chain is unsupported.

## CLI Commands

```bash
safe-competition-trading-agent plan --input request.json --format json
safe-competition-trading-agent dry-run --input request.json --format json
safe-competition-trading-agent execute --input request.json --format json
safe-competition-trading-agent validate --input plan.json --format json
safe-competition-trading-agent template --name competition-safe-swap
safe-competition-trading-agent self-test
```

The CLI never signs or broadcasts. It creates plans, dry-run decisions, guarded execution commands, and local validation reports.

## Input Contract

```json
{
  "intent": "Dry-run a safe competition trade with a 10 USD budget.",
  "chain": "xlayer",
  "goal": "safe-volume",
  "budgetUsd": "10",
  "tokenIn": {"symbol": "USDC"},
  "executionMode": "dry-run",
  "confirmed": false,
  "competition": {
    "activityName": "Selected Agentic Trading competition",
    "active": true,
    "joined": true,
    "chainName": "X Layer",
    "supportedChains": ["xlayer", "solana"],
    "eligibleTokenTradeRequired": true
  },
  "candidates": [
    {
      "symbol": "MEME",
      "address": "0x...",
      "liquidityUsd": "150000",
      "volume24hUsd": "75000",
      "riskLevel": "LOW"
    }
  ],
  "quote": {
    "expectedOut": "12345",
    "slippagePct": 1,
    "priceImpactPct": 1
  },
  "riskVerdict": {
    "verdict": "allow",
    "reasons": []
  }
}
```

Supported values:

- `chain`: `xlayer`, `solana`
- `goal`: `safe-volume`, `rank-optimizer`, `eligible-token`, `custom`
- `executionMode`: `dry-run`, `confirm-before-execute`

## Execution Flow

Follow this exact order:

1. Parse user intent: chain, budget, goal, dry-run vs execution.
2. Run `onchainos wallet status`.
3. Run `onchainos competition list --status 0`.
4. Select the competition by user intent or ask the user to choose by name.
5. Run `onchainos competition detail --activity-id <activityId>`.
6. Run `onchainos competition user-status --activity-id <activityId> --evm-wallet <evmWallet> --sol-wallet <solWallet>`.
7. If the user has not joined, warn that the trade may not count. Ask whether to join before trading.
8. Query onchainOS token data for candidate tokens on the selected chain.
9. Exclude stable/native-only candidates for competition mode unless the user explicitly asks for a non-eligible dry-run.
10. Run `onchainos swap quote`.
11. Prepare unsigned swap transaction context through onchainOS only after the user has approved the dry-run preparation step. This preparation step is not live execution and must not sign or broadcast.
12. Run `agent-workflow-composer plan` with `workflowType: "competition-trade"`.
13. Run `agent-risk-firewall check` with `policyProfile: "competition"`.
14. Apply the verdict:
    - `block`: stop. Do not ask for signature.
    - `warn`: show reasons and ask for explicit confirmation.
    - `allow`: continue only if the user already requested execution.
15. If execution is confirmed, run `onchainos swap execute`.
16. Show tx result, risk decision id, and next step.

## Output Format

For user-facing output, use this structure:

```md
## Competition Trade Plan

Competition: <name only, no internal id>
Chain: <X Layer or Solana>
Mode: <dry-run or confirm-before-execute>
Budget: <amount and token>

## Candidate

Token: <symbol>
Reason:
- <selection reason>

## Quote

Route: <route>
Estimated out: <amount>
Slippage: <percent>
Price impact: <percent>

## Risk Verdict

Verdict: <allow | warn | block>
Risk score: <0-100>
Reasons:
- <reason code and short message>

## Next Step

<continue, ask confirmation, join competition first, or cancel>
```

Do not show `activityId`, `accountId`, private keys, seed phrases, mnemonics, raw secrets, or internal-only identifiers.

## Safety Rules

- Default to `dry-run`.
- Never execute when `agent-risk-firewall` returns `block`.
- Never execute on `warn` without explicit user confirmation.
- Never execute if `competition_user_status` is missing.
- Never execute if quote or unsigned transaction context is missing.
- Never use `--force`.
- Never export or request private keys, seed phrases, or mnemonics.
- Never claim that a trade will definitely win a competition.

## Error Handling

| Situation | Response |
|---|---|
| Wallet not logged in | Ask user to run or approve wallet login before trading. |
| No active competition | Stop and show that no active competition was found. |
| User not joined | Warn and ask whether to join before trading. |
| No token candidate | Query onchainOS token/signal data; if still none, stop. |
| Quote unavailable | Retry once; if still unavailable, stop. |
| Firewall unavailable | Treat as `warn` or stop in strict review contexts; do not execute blindly. |
| Firewall block | Cancel trade. |

## Example Agent Behavior

User: "Trade 10 dollars for a safe competition token on X Layer."

Agent should:

1. Explain it will dry-run first.
2. Fetch wallet and competition context through onchainOS.
3. Select a real token candidate from onchainOS data.
4. Quote and prepare unsigned tx context through onchainOS.
5. Run workflow composer and risk firewall.
6. Show the structured output.
7. Ask for explicit confirmation only if the trade is not blocked.

## Disclaimer

This skill is a trading guardrail and strategy workflow, not a guarantee of profit or safety. On-chain data, competition rules, rankings, quotes, and simulations can change or be stale. Trading can cause loss of funds.
