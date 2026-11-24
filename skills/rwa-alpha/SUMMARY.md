## Overview

RWA Alpha is a Real World Asset intelligence trading skill that combines macro event detection, Polymarket probability confirmation, and on-chain price action to auto-trade tokenized treasury, gold, yield, and governance tokens via OKX DEX.

Core operations:

- Detect macro events (rate decisions, credit cycles, inflation) from NewsNow RSS feeds
- Confirm signals with Polymarket prediction market probability as a filter
- Track NAV premium/discount for tokenized RWA assets and trade accordingly
- Execute trades on Ethereum and Solana via onchainos Agentic Wallet (TEE signing)
- Monitor positions, macro feed, and yield rankings on a live web dashboard

Tags: `rwa` `real-world-assets` `macro` `treasury` `gold` `onchainos` `ethereum` `solana`

## Prerequisites

- No IP/region restrictions
- Supported chains: Ethereum, Solana
- Supported tokens: USDY, OUSG, bIB01, STBT, PAXG, ONDO, CFG, PENDLE, PLUME, OM, GFI, TRU
- onchainos CLI installed and authenticated (`onchainos --version` and `onchainos wallet status`)
- Python 3.8+ (standard library only — no `pip install` required)
- (Optional) Anthropic API key for AI-enhanced macro event classification
- Sufficient balance on Ethereum or Solana for RWA token trading

## Quick Start

1. **Install the skill**: `plugin-store install rwa-alpha`
2. **Choose your mode**: Set `MODE` in `config.py` — `YIELD_OPTIMIZER` (conservative), `MACRO_TRADER` (balanced), or `FULL_ALPHA` (aggressive)
3. **Start in paper mode** (default, `PAPER_TRADE = True`): Run `python3 rwa_alpha.py`
4. **Monitor signals**: Open the web dashboard to view macro events, Polymarket probabilities, and NAV premium tracking
5. **Review positions**: Check that entries and exits match your expected strategy behavior over 1–2 sessions
6. **Go live**: Set `PAPER_TRADE = False` in `config.py` and restart — confirm wallet balance and risk limits before switching
