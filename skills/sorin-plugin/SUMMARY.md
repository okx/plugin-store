## Overview

Sorin Skill routes DeFi questions about tokens, pools, chains, protocols, and projects to Sahara's Sorin DeFi AI Services Gateway. It helps agents choose the right analysis endpoint, call it with explicit parameters, and summarize the returned data with assumptions and risks.

## Prerequisites

- `DEFI_TOOLS_API_KEY` set in the environment.
- Network access to `https://defi-tools-proxy.saharaa.info`.
- Required user inputs such as token symbol, protocol name, project name, chain name, or pool filters.

## Quick Start

1. For token analysis, ask about a token symbol such as `ETH` or `BTC`.
2. For yield or staking analysis, provide a chain, protocol, token symbol, or pool ID when available.
3. For protocol or project analysis, provide the protocol or project name.
4. The skill selects the matching gateway endpoint, calls it with the available parameters, and returns key findings, interpretation, next steps, and caveats.
