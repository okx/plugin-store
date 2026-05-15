# Agent Workflow Composer

## Overview

Agent Workflow Composer builds explicit workflow plans for Agentic Wallet tasks that need multiple plugins. It solves the missing composition layer between strategy plugins, OnchainOS skills, and `agent-risk-firewall`.

Core operations:

- Build a guarded workflow plan from a natural-language intent request
- Validate that risk checks and user confirmation happen before execution
- Generate templates for guarded swaps, competition trades, and approval reviews
- Add Competition Mode Enhancer steps for OKX competition discovery, detail, user-status, and firewall competition context
- Keep execution disabled by default through `dry-run` mode

Tags: `workflow` `agentic-wallet` `composer` `security` `risk` `trading`

## Prerequisites

- Python 3.8+
- Optional plugins for real workflows:
  - `okx-agentic-wallet`
  - `okx-dex-swap`
  - `okx-dex-token`
  - `agent-risk-firewall`
  - `okx-growth-competition` for competition-trade workflows
  - optional external evidence plugins such as `goplus-security`, `birdeye-plugin`, or `rootdata-crypto-plugin`

## Quick Start

1. **Create a request** with `intent`, `workflowType`, `executionMode`, and `riskProfile`.
2. **Run composer** with `agent-workflow-composer plan --input request.json --format json`.
3. **Validate the plan** with `agent-workflow-composer validate --input plan.json --format json`.
4. **Give the plan to an agent** so it follows the ordered steps and safety gates.

The plugin does not sign, broadcast, execute swaps, move assets, access wallets, or handle private keys.
