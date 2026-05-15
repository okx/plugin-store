# Agent Risk Firewall

## Overview

Agent Risk Firewall is a pre-trade guardrail for Agentic Wallet swaps on X Layer and Solana. It receives a proposed intent, quote, transaction context, approval context, and optional external evidence, runs available OKX OnchainOS checks, applies a deterministic policy profile, and returns `allow`, `warn`, or `block`.

Core operations:

- Check proposed swaps, token buys, token sells, and approvals before signing
- Normalize OKX security scan, transaction scan, token report, and simulation evidence
- Aggregate optional GoPlus, Birdeye, and RootData evidence supplied by other plugins
- Check approval spender and unlimited allowance risk
- Apply Competition Mode Enhancer checks for Agentic Trading workflows: active status, join status, supported chains, thresholds, and eligible token pairs
- Return audit fields: `decisionId`, `policyVersion`, and `evidenceHash`
- Return deterministic `allow`, `warn`, or `block` verdicts for agent workflows

Tags: `security` `risk` `agentic-wallet` `xlayer` `solana` `trading`

## Prerequisites

- No IP restrictions enforced by this plugin
- Supported chains: X Layer and Solana
- Supported operations: swaps, token buys, token sells, and approvals
- Python 3.8+
- Optional for live checks: `npx skills add okx/onchainos-skills`
- Optional for production reliability: personal OKX OnchainOS API credentials configured in the environment

## Quick Start

1. **Prepare a request**: Build a JSON payload with `chain`, `operation`, wallet address, token context, quote details, and optional transaction context.
2. **Run the firewall**: Execute `agent-risk-firewall check --input request.json --format json` before asking the user to sign.
3. **Apply the verdict**: Continue on `allow`, ask explicit confirmation on `warn`, and cancel the operation on `block`.
4. **Pick a policy profile**: Use `balanced`, `strict`, `competition`, or `degen-small-size` depending on the workflow. For `competition`, fetch OKX competition detail and user-status first and pass the normalized `competition` context.
5. **Inspect policy or installation**: Use `agent-risk-firewall policy --profile balanced` to view thresholds and `agent-risk-firewall self-test` to verify local installation.

The plugin does not sign, broadcast, trade, revoke approvals, or handle private keys.
