## Overview

RootData OKX Edition - a read-only Web3 data lookup skill covering crypto projects,
investors, funding rounds, trending projects, and personnel job changes.

Core operations:[SKILL.md](SKILL.md)

- Search projects, VCs, and people by keyword
- Look up project details, investors, funding history
- Track trending projects (1-day / 7-day windows)
- Monitor crypto industry hires and departures

Tags: `crypto` `web3` `research` `funding` `rootdata`

## Prerequisites

- No IP/region restrictions (public data API)
- `OKX_ROOTDATA_SKILL_KEY` environment variable (auto-provisioned via /init on first run)
- Internet access to api.rootdata.com
- Rate limit: 200 req/min per key

## Quick Start

1. **First-time setup**: ask the agent anything that uses RootData. If
   `OKX_ROOTDATA_SKILL_KEY` is missing, the skill calls `/init` to mint a key and
   stores it as an env var.
2. **Search**: e.g., "find Uniswap on RootData" -> returns entity ID.
3. **Detail lookup**: pass the ID to project/funding endpoints for full data.
4. **Trending / job changes**: ask "what's hot today" or "who joined which crypto
   project this week".
