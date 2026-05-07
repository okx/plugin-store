## Overview

Macro Intelligence is a unified macro signal feed that reads 7 data sources, classifies macro events, scores market sentiment, and exposes real-time regime signals via a local HTTP API for other skills and agents to consume.

Core operations:

- Ingest macro events from 7 sources (Fed, CPI, gold, tariffs, whale flows, and more)
- Classify events by type (rate decision, credit expansion, risk-on/off regime)
- Score sentiment and generate AI insights per event
- Expose live signals via a local HTTP API at `http://localhost:3260/signals`
- Feed signals into downstream skills (e.g. rwa-alpha for trade confirmation)

Tags: `macro` `sentiment` `news` `signals` `fed` `cpi` `gold` `onchainos`

## Prerequisites

- No IP/region restrictions
- No wallet or on-chain operations required (read-only signal feed)
- Python 3.8+ (standard library only — no `pip install` required)
- (Optional) Anthropic API key for AI-generated insights (`ANTHROPIC_API_KEY` env var)
- Downstream skills (e.g. rwa-alpha) must be configured to point to `http://localhost:3260`

## Quick Start

1. **Install the skill**: `plugin-store install macro-intelligence`
2. **Start the feed**: Run `python3 macro.py` — it begins ingesting sources immediately
3. **Query signals**: `curl http://localhost:3260/signals` to see the latest classified macro events and sentiment scores
4. **Connect downstream skills**: In `rwa-alpha/config.py`, set `MACRO_API = "http://localhost:3260"` to use this feed as a signal gate
5. **Monitor the feed**: Check `http://localhost:3260` in your browser for a real-time view of macro events and regime scores
