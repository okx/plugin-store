---
name: cross-hedge-alpha
description: "Cross-protocol delta-neutral hedging strategy bridging Hyperliquid perps and Polymarket prediction events."
license: MIT
metadata:
  author: Lino Alemz
  version: "1.0.0"
  strategyId: "CH_ALPHA_2026_S1"
---

# Cross-Hedge Alpha: Multi-Protocol Risk Management

The most advanced agentic strategy on Onchain OS. It bridges Perpetual Futures (Hyperliquid) and Prediction Markets (Polymarket).

## 🟢 Pilot Mode (Safety Automation)
Automatically adjust hedges asperp positions grow or market conditions change.
- **Trigger**: "Auto-hedge my portfolio" or "Enable Cross-Hedge Pilot"
- **Logic**: Re-calculates Delta every 15 minutes.

## Intent Triggers
"hedge my HL with polymarket", "delta neutral strategy", "protect my longs", "arbitrage HL and poly", "advanced risk management"

## Strategy Logic
1. **Exposure Analysis**: The agent reads the user's active Hyperliquid positions via `okx-wallet-portfolio` and `hyperliquid-plugin`.
2. **Hedge Identification**: Finds a Polymarket event that is inversely correlated with the perp position (e.g., if Long ETH, find a market for "Will ETH price drop?").
3. **Delta Calculation**: Calculates the exact bet size needed on Polymarket to offset the perp's downside.
4. **Execution**: Opens the Polymarket position.

## Command Reference

### 1. `onchainos hedge auto --ratio <val>`
Automatically scans for your largest HL position and hedges it with a correlated Polymarket event.

### 2. `onchainos hedge scan`
Returns a list of potential cross-protocol arbitrage or hedge opportunities.

## Risk Controls
- **Liquidity Check**: Only hedges using markets with >$100k liquidity.
- **Slippage Protection**: Cancels the hedge if Polymarket odds move >5% during execution.

---
*Created for OKX Plugin Store Season 1 Developer Challenge.*
