## Overview

Sorin Skill routes DeFi questions about tokens, pools, chains, protocols, and projects to Sahara's Sorin DeFi AI Services Gateway. It helps agents choose the right analysis endpoint, call it with explicit parameters, and summarize the returned data with assumptions and risks.

## Prerequisites

- `DEFI_TOOLS_API_KEY` set in the environment.
- Network access to `https://tools.saharaai.com/api`.
- Required user inputs such as token symbol, protocol name, project name, chain name, or pool filters.

## Quick Start

1. Run `sorin-skill quickstart` to verify `DEFI_TOOLS_API_KEY` is set and the gateway is reachable.
2. Ask a token question using a symbol such as `ETH` or `BTC`.
3. For yield or staking analysis, provide a chain, protocol, token symbol, or pool ID when available.
4. For protocol or project analysis, provide the protocol or project name.
5. Review the returned findings, assumptions, risks, next steps, and caveats.
