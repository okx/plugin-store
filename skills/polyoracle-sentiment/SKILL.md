---
name: polyoracle-sentiment
description: "AI-driven sentiment analysis for Polymarket. Automatically bets based on social signals and trending news."
license: MIT
metadata:
  author: Lino Alemz
  version: "1.0.0"
  strategyId: "PO_SENT_2026_S1"
---

# PolyOracle: Sentiment-Driven Predictions

This skill enables users to place bets on Polymarket using AI-analyzed social signals and whale activity.

## 🟢 Pilot Mode (Social Automation)
Enable the agent to automatically place small bets on high-confidence social signals.
- **Trigger**: "Auto-bet on social sentiment" or "Enable PolyOracle Pilot"
- **Limit**: Default max $10 per bet for safety.

## Intent Triggers
"bet on sentiment", "what does the social sentiment say about the election", "polymarket whale tracker", "is it a good time to bet on X", "automated prediction betting"

## Command Reference

### 1. `onchainos poly analyze --market <slug>`
Fetches the latest social signals for the given market and returns a **Sentiment Score (0-100)**.

### 2. `onchainos poly auto-bet --threshold <val>`
Automatically places a $5 bet on any market where sentiment > threshold.

## Strategy Logic
1. **Signal Aggregation**: The skill calls `okx-dex-signal` to find what KOLs and Whales are buying in the corresponding token markets (e.g., if Whales are buying $TRUMP token, it signals positive sentiment for the Trump Polymarket).
2. **Confidence Weighting**: Combines signal strength with market liquidity.
3. **Execution**: Uses `polymarket-plugin` to execute the transaction.

## Risk Controls
- **Max Bet Limit**: Default $10 per market to prevent heavy losses on false signals.
- **Divergence Warning**: Warns if social sentiment and Polymarket odds are heavily divergent (>20%).

---
*Created for OKX Plugin Store Season 1 Developer Challenge.*
