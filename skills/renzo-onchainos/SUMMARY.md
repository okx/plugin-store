# renzo-onchainos

## Overview

Renzo Protocol is a liquid restaking protocol on Ethereum that wraps native ETH or LSTs into ezETH, a liquid restaking token (LRT) that earns EigenLayer restaking rewards plus additional yield. This plugin wraps the Renzo API to build `pending_sign` deposit transactions and track withdrawal requests, routed through Onchain OS for TEE-based signing.

Core operations:
- Get current APR and protocol statistics
- Get the current ezETH/ETH exchange rate
- Preview withdrawal amounts before redeeming
- List pending withdrawal requests and their status
- Build ETH deposit transactions to receive ezETH (returns `pending_sign` envelope)

Tags: `restaking` `liquid-staking` `renzo` `ezeth` `eigenlayer` `ethereum`

## Prerequisites

- No IP restrictions
- Supported chain: Ethereum (1)
- Supported tokens: ETH (native), ezETH
- onchainos CLI installed and authenticated
- Node.js ≥ 18 and `tsx` for running the skill (`npm install` in skill directory)

## Quick Start

1. **Install dependencies**: `cd ~/.agents/skills/renzo-onchainos && npm install`
2. **Check rates**: Ask Claude "what is the current Renzo ezETH APR?"
3. **Preview deposit**: Ask Claude "how much ezETH would I get for 0.1 ETH on Renzo?"
4. **Deposit ETH**: Claude will call `buildDepositEth`, return a `pending_sign` envelope, and route it through `onchainos wallet contract-call` for signing and broadcast
