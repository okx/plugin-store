# Safe Competition Trading Agent

## Overview

Safe Competition Trading Agent is a competition-aware Agentic Wallet strategy skill. It uses onchainOS competition, wallet, token, quote, and swap commands as the main data and trading layer, then applies workflow and risk gates before any live execution.

Core operations:

- Plan a competition trade from a user intent, chain, budget, and goal
- Fetch competition detail and user registration status through onchainOS
- Select safer real-token candidates from onchainOS token context
- Prepare quote and unsigned transaction context through onchainOS
- Validate workflow order with `agent-workflow-composer`
- Enforce risk verdicts with `agent-risk-firewall`
- Require explicit user confirmation before live execution

Tags: `trading` `competition` `agentic-wallet` `xlayer` `solana` `security` `risk`

## Prerequisites

- Python 3.8+
- Agentic Wallet login for live execution
- onchainOS CLI/skills installed
- Recommended supporting plugins:
  - `okx-agentic-wallet`
  - `okx-growth-competition`
  - `okx-dex-token`
  - `okx-dex-swap`
  - `agent-workflow-composer`
  - `agent-risk-firewall`

## Quick Start

1. **Build a plan** with `safe-competition-trading-agent plan --input request.json --format json`.
2. **Dry-run first** with competition, quote, and risk context.
3. **Review risk verdict** from `agent-risk-firewall`.
4. **Execute only after explicit confirmation** and only when the firewall allows or warns with confirmed consent.

The plugin does not sign, broadcast, export wallets, or handle private keys by itself.
