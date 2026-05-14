## Overview

X Layer Alpha Hunter is an intelligent trading strategy plugin built on OKX Onchain OS that monitors Smart Money (institution-level wallets) on the X Layer blockchain to capture alpha signals and assist with automated copy trading.

Core operations:

- **Smart Money Signal Monitoring**: Track OKX pre-classified Smart Money wallets (walletType=1) on X Layer, capturing real-time buy/sell behaviors via `onchainos signal list` and `onchainos tracker activities`
- **Alpha Signal Identification**: Dual-filter using convergence_score and alpha_score to identify high-confidence signals (SM wallets >= 4, convergence_score >= 5.0, alpha_score >= 6.0)
- **New Token Alpha Discovery**: Monitor low-market-cap tokens with zero SM presence for early埋伏 (pre-positioning) opportunities
- **Automated Trading Execution**: Dry-run first design with safe auto-sell triggers (soldRatio=100%, profit >= 50%, or stop-loss at -15%)
- **Daily Review Reports**: Auto-generate trade logs and strategy analysis reports

This plugin prioritizes **dry-run mode** — real trading requires explicit `tradable=true` configuration.

Tags: `x-layer` `smart-money` `auto-trade` `alpha` `onchainos`

## Prerequisites

- **No IP restrictions** — Available globally
- **Supported chain**: X Layer (chainIndex=196)
- **Required tools**: onchainos CLI installed and authenticated with OKX wallet
- **Optional**: Telegram Bot configured for Hermes Agent (for real-time push notifications)
- **Minimum balance**: A small amount of X Layer native tokens for gas

## Quick Start

1. **Install and authenticate onchainos CLI**
   ```bash
   npx skills add okx/onchainos-skills
   onchainos wallet status  # Confirm logged in
   ```

2. **Run in monitor mode (recommended first)**
   ```bash
   python3 /root/scripts/run_xlayer_trading.py --mode monitor
   ```
   This displays all SM signals without executing any trades.

3. **Review daily signals**
   ```bash
   python3 /root/scripts/run_xlayer_trading.py --review
   ```
   Generates yesterday's trading report and strategy analysis.

4. **Enable auto-trading (after monitor mode confirms signal quality)**
   ```bash
   # Set TRADABLE=true before running
   python3 /root/scripts/run_xlayer_trading.py --mode auto
   ```

**Key strategy parameters**: convergence_threshold=5.0, alpha_threshold=6.0, min_sm_count=4, stop_loss=-15%, take_profit=50%

**Risk disclaimer**: This plugin is for informational purposes only. Always use dry-run mode first. Cryptocurrency trading involves significant risk.
