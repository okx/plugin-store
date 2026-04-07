---
name: top-rank-tokens-sniper
description: >
  Top Rank Tokens Sniper v1.0 — OKX Ranking Sniper (Real Trading).
  Monitors the OKX leaderboard for newly listed tokens, filters through
  13 Slot Guard pre-checks + 9 Advanced Safety checks +
  3 Holder Risk checks + Momentum scoring, then automatically snipes entries.
  6-layer exit system manages take profit and stop loss.
  Triggered when the user mentions top rank tokens sniper, ranking strategy,
  leaderboard sniper, top N sniper, 榜单狙击手, or start ranking sniper.
  Run file: ranking_sniper.py (includes Web Dashboard http://localhost:3244)

version: 1.0
updated: 2026-03-26
---


## Pre-flight Dependencies (auto-injected by Plugin Store CI)

> Run once per session before first use. These checks ensure required tools are installed.

### Install onchainos CLI + Skills (auto-injected)

```bash
# 1. Install onchainos CLI
onchainos --version 2>/dev/null || curl -fsSL https://raw.githubusercontent.com/okx/onchainos-skills/main/install.sh | sh

# 2. Install onchainos skills (enables AI agent to use onchainos commands)
npx skills add okx/onchainos-skills --yes --global

# 3. Install plugin-store skills (enables plugin discovery and management)
npx skills add okx/plugin-store --skill plugin-store --yes --global
```

### Report install (auto-injected, runs once)

```bash
REPORT_FLAG="$HOME/.plugin-store/reported/top-rank-tokens-sniper"
if [ ! -f "$REPORT_FLAG" ]; then
  mkdir -p "$HOME/.plugin-store/reported"
  # Device fingerprint → SHA256 → 32-char device ID
  DEV_RAW="$(hostname):$(uname -s):$(uname -m):$HOME"
  DEV_ID=$(echo -n "$DEV_RAW" | shasum -a 256 | head -c 32)
  # HMAC signature (obfuscated key, same as CLI binary)
  _K=$(echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | base64 -d 2>/dev/null || echo 'OE9nNWFRUFdfSVJkektrMExOV2RNeTIzV2JibXo3ZWNTbExJUDFIWnVoZw==' | openssl base64 -d)
  HMAC_SIG=$(echo -n "${_K}${DEV_ID}" | shasum -a 256 | head -c 8)
  DIV_ID="${DEV_ID}${HMAC_SIG}"
  unset _K
  # Report to Vercel stats
  curl -s -X POST "https://plugin-store-dun.vercel.app/install" \
    -H "Content-Type: application/json" \
    -d '{"name":"top-rank-tokens-sniper","version":"1.0.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"top-rank-tokens-sniper","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


# Top Rank Tokens Sniper v1.0

> This is a real trading bot. Make sure you understand the risks before use. It is recommended to test in Paper mode first.

---

## Disclaimer

**This strategy script, parameter configuration, and all related documentation are provided solely for educational research and technical reference purposes. They do not constitute any form of investment advice, trading guidance, or financial recommendation.**

1. **High Risk Warning**: Cryptocurrency trading (especially on-chain Meme tokens) carries extremely high risk. Prices may fluctuate drastically within seconds or even go to zero. You may lose your entire invested capital.
2. **Ranking Data Risk**: Leaderboard ranking data may be manipulated by wash trading. Changes in ranking do not represent genuine market consensus. Trading decisions based on rankings may result in losses due to data distortion.
3. **Parameters for Reference Only**: All default parameters in this strategy (take profit, stop loss, position size, safety thresholds, etc.) are set for general scenarios and **are not guaranteed to be suitable for any specific market conditions**. Users should adjust all parameters according to their own risk tolerance, trading experience, and market judgment.
4. **User Customization**: Users are encouraged to deeply understand the meaning of each parameter and modify them according to their own strategy logic and risk preferences. Every parameter in `config.py` is annotated with comments for easy customization.
5. **No Guaranteed Returns**: Past performance does not represent future results. Even parameters that perform well in backtesting may fail in live trading due to changing market conditions.
6. **Technical Risk**: On-chain transactions are irreversible. Smart contracts may contain vulnerabilities. Network congestion may cause transaction delays or failures.
7. **Third-Party Dependency Risk**: This strategy relies on third-party infrastructure including onchainos CLI, OKX API, and the Solana network. Their availability, accuracy, and stability are beyond the strategy author's control. Any changes, interruptions, or failures in these services may cause the strategy to malfunction or produce unexpected losses.
8. **Regulatory/Legal Risk**: Cryptocurrency trading may be strictly restricted or prohibited in some countries and regions. Users should understand and ensure compliance with all applicable laws and regulations in their jurisdiction before using this strategy.
9. **Tax Risk**: Frequent trading may generate numerous taxable events. Users should understand and comply with local tax laws regarding reporting and paying taxes on cryptocurrency trading gains.
10. **Assume All Responsibility**: This strategy is provided "AS-IS" without any express or implied warranties. All trading decisions made using this strategy and their consequences are the sole responsibility of the user. The strategy authors, developers, distributors, and their affiliates are not liable for any direct, indirect, incidental, or special losses.

**Recommendation**: For first-time use, please run in Paper mode (`MODE = "paper"`). After fully familiarizing yourself with the strategy logic and parameter behavior, then consider whether to switch to live trading.

---

## File Structure

```
Top Rank Tokens Sniper - 榜单狙击手/
├── skill.md              ← This file (strategy documentation)
├── config.py             ← All adjustable parameters (modify parameters here only)
├── ranking_sniper.py     ← Main strategy program
├── dashboard.html        ← Web Dashboard UI
└── state/                ← [Auto-generated] Runtime data
    ├── paper/
    │   ├── positions.json
    │   ├── trades.json
    │   ├── daily-stats.json
    │   └── signals-log.json
    └── live/
        └── (same as above)
```

---

## Prerequisites

### 1. Install onchainos CLI (>= 2.0.0-beta)

```bash
# Check if already installed
onchainos --version

# If not installed, follow the onchainos official documentation
# Make sure onchainos is in PATH or located at ~/.local/bin/onchainos
```

### 2. Login to Agentic Wallet (TEE Signing)

```bash
# One-time login (email verification)
onchainos wallet login <your-email>

# Verify login status
onchainos wallet status
# → loggedIn: true

# Confirm Solana address
onchainos wallet addresses --chain 501
```

> Agentic Wallet uses TEE secure enclave signing. Private keys are never exposed to code/logs/network.
> No need to set the WALLET_PRIVATE_KEY environment variable.

### 3. No pip install Required

This strategy only depends on Python standard library + onchainos CLI. No third-party packages needed.

---

## AI Agent Startup Interaction Protocol

> **When the user requests to start this strategy, the AI Agent must follow the procedure below and must not skip directly to launch.**

### Phase 1: Show Strategy Overview

Present the following to the user:

```
🏆 Top Rank Tokens Sniper v1.0 — Solana Ranking Sniper

This strategy scans the Solana 1h gainers leaderboard Top 20 every 10 seconds.
When a new token first appears on the leaderboard, it passes through
three-level safety filtering + Momentum scoring, then automatically snipes entry.
Positions are managed through a 6-layer exit system.

🧪 Current: Paper Mode — no real money spent, observing signals only

⚠️ Risk Warning: Meme coins carry extremely high risk. You may lose your entire investment.

Default parameters (for reference only, recommended to adjust based on your situation):
  Per trade:        0.05 SOL
  Total budget:     0.5 SOL
  Max positions:    5
  Take profit:      TP1 +8% / TP2 +20% / TP3 +40%
  Stop loss:        -15% Hard Stop / -8% Quick Stop (3min)
  Trailing stop:    Activates at +10% profit, exits on 8% drawdown
  Ranking exit:     Auto sell 100% when dropped out of Top 20 (highest priority)
  Max hold time:    2 hours

All parameters can be freely modified in config.py to suit your trading style.
```

### Q1: Risk Preference (Mandatory)

- 🛡️ Conservative: Quick in-and-out, small TP with tight SL
- ⚖️ Default: Balanced configuration (recommended)
- 🔥 Aggressive: Large TP with wide SL

→ Parameter mapping (for AI Agent to write into config.py, no need to show to user):

| Preference | STOP_LOSS_PCT | QUICK_STOP_MIN | QUICK_STOP_PCT | TP_TIERS | MAX_HOLD_HOURS | TRAILING_ACTIVATE | TRAILING_DROP |
|------|--------------|----------------|----------------|----------|----------------|-------------------|---------------|
| Conservative | -10 | 2 | -5 | (5,0.30),(12,0.35),(25,0.35) | 1 | 8 | 5 |
| Default | -15 | 3 | -8 | (8,0.30),(20,0.35),(40,0.35) | 2 | 10 | 8 |
| Aggressive | -25 | 5 | -12 | (12,0.30),(30,0.35),(60,0.35) | 4 | 15 | 12 |

### Q2: Switch to Live Trading?

- A. 🧪 Keep Paper mode, start directly (recommended by default)
- B. 💰 Switch to Live mode

**Option A** → Proceed directly to the launch step.

**Option B** → Enter live trading sub-flow:

1. ⚠️ Confirm with user:
   "Live trading will use real SOL. Losses are irreversible. Confirm switch to live?"
   - User confirms → Continue
   - User declines → Fall back to Paper mode

2. Ask for total budget in SOL (default 0.5 SOL)

3. AI auto-calculates (let B = user's input budget):
   - `TOTAL_BUDGET = B`
   - `BUY_AMOUNT = max(B × 0.10, 0.01)`

4. Show calculated results and confirm with user:
   "Your live configuration: Total budget X SOL, per trade Y SOL, daily loss limit Z SOL. Confirm?"
   - User confirms → Write to config.py
   - User requests adjustment → Return to step 2

5. Set mode parameters:
   - `MODE = "live"`
   - `PAUSED = False`

### Launch

1. Modify corresponding parameters in `config.py` based on user answers
2. Set `PAUSED = False` (allow bot to run normally after interaction confirmation)
3. Check prerequisites: `onchainos --version`, `onchainos wallet status`
4. Start bot: `python3 ranking_sniper.py`
5. Show Dashboard link: `http://localhost:3244`
6. Inform user: Currently in Paper mode. To switch to live, modify `MODE = "live"` in `config.py`

If the user says "use default config" or "just run it", only set `PAUSED = False`, leave everything else unchanged, and start in Paper mode.

### Special Cases

- User explicitly says "don't ask me, just run it" → Start with default parameters (Paper mode), but must show Phase 1 overview + set `PAUSED = False`
- User is a returning user (configuration history exists in conversation) → Remind them of previous configuration and ask if they want to reuse it

---

## Quick Start

> ⚠️ Before starting, confirm the `MODE` value in config.py — `"paper"` for Paper trading, `"live"` for Live trading.

```bash
cd ~/CC/Top\ Rank\ Tokens\ Sniper\ -\ 榜单狙击手

# 1. Confirm onchainos is logged in
onchainos wallet status

# 2. Start bot (foreground, Ctrl+C to stop)
python3 ranking_sniper.py

# 3. Open Dashboard
open http://localhost:3244

# 4. Stop
pkill -f ranking_sniper.py
```

> **First startup defaults to PAUSED=True — no new positions will be opened. After confirming everything is normal, modify PAUSED=False in config.py.**

---

## Parameter Adjustment

**All adjustable parameters are in `config.py`** — no need to modify `ranking_sniper.py`.

### Common Adjustments

| Need | Modify in `config.py` |
|---|---|
| Pause/resume trading | `PAUSED = True/False` |
| Adjust per-trade amount | `BUY_AMOUNT = 0.05` |
| Adjust total budget | `TOTAL_BUDGET = 0.5` |
| Adjust max positions | `MAX_POSITIONS = 5` |
| Adjust take profit | `TP_TIERS = [(8,0.30),(20,0.35),(40,0.35)]` |
| Adjust hard stop loss | `STOP_LOSS_PCT = -15` |
| Adjust quick stop | `QUICK_STOP_MIN = 3`, `QUICK_STOP_PCT = -8` |
| Adjust trailing stop | `TRAILING_ACTIVATE = 10`, `TRAILING_DROP = 8` |
| Adjust sell slippage | `SLIPPAGE_SELL = 8` (normal exit), `SLIPPAGE_SELL_URGENT = 15` (urgent exit) |
| Adjust scan speed | `POLL_INTERVAL = 10` (seconds) |
| MC range | `MIN_MCAP = 50_000`, `MAX_MCAP = 10_000_000` |
| Paper trading | `MODE = "paper"` |
| Dashboard port | `DASHBOARD_PORT = 3244` |

Restart the bot for changes to take effect.

---

## Strategy Architecture

```
ranking_sniper.py (Single-file Bot)
├── onchainos CLI (Data + Execution + Security — no API Key)
├── _scanner_loop()    ← Background thread, every 10s
│   ├── get_ranking()          Leaderboard Top 20
│   ├── New entry detection    prev_snap set diff
│   └── _filter()              Three-level filtering
│       ├── Level 1: Slot Guard (13 basic metrics)
│       ├── Level 2: Advanced Safety (9 safety checks)
│       ├── Level 3: Holder Risk Scan (3 holder risk checks)
│       ├── _calc_score()      Momentum Score calculation
│       └── → _buy() (synchronous execution)
│           └── Live mode 4-layer verification
├── _monitor_loop()    ← Background thread, every 10s
│   ├── get_batch_prices()     Batch prices
│   ├── _check_unconfirmed()   Layer 3 monitoring
│   └── check_position()       Exit decisions
│       ├── EXIT 0: Ranking Exit (dropped off leaderboard)
│       ├── EXIT 1: Hard Stop (-15%)
│       ├── EXIT 2: Quick Stop (3min, -8%)
│       ├── EXIT 3: Trailing Stop (peak +10%, drop 8%)
│       ├── EXIT 4: Time Stop (2h)
│       └── EXIT 5: Tiered TP (+8%/+20%/+40%)
├── _audit_loop()      ← Background thread, every 5min (Live mode)
│   └── _wallet_audit()        Wallet reconciliation
├── Dashboard (port 3244)      Web UI
└── Persistent files (JSON, atomic writes)
```

---

## Safety Checks

### Level 1: Slot Guard (13 checks, based on leaderboard data)

| # | Check | Threshold |
|---|---|---|
| 1 | Min price change | >= 15% |
| 2 | Max price change | <= 500% |
| 3 | Liquidity | >= $30,000 |
| 4 | Market cap floor | >= $50,000 |
| 5 | Market cap ceiling | <= $10M |
| 6 | Holders | >= 100 |
| 7 | Buy ratio | >= 55% |
| 8 | Unique traders | >= 20 |
| 9 | Blacklist | Not in SKIP_TOKENS/BLACKLIST |
| 10 | Cooldown | >= 30min since last sell |
| 11 | Position cap | < MAX_POSITIONS |
| 12 | Dedup | Not already holding same token |
| 13 | Daily loss | Daily loss limit not triggered |

### Level 2: Advanced Safety (9 checks, onchainos token advanced-info)

| # | Check | Threshold |
|---|---|---|
| S1 | Risk level | <= 3 |
| S2 | Honeypot | No honeypot tag |
| S3 | Top 10 concentration | <= 40% |
| S4 | Dev holding | <= 15% |
| S5 | Bundler holding | <= 15% |
| S6 | LP burned | >= 50% |
| S7 | Dev rug count | <= 2 |
| S8 | Sniper holding | <= 15% |
| S9 | Internal token | Default pass |

### Level 3: Holder Risk (3 checks, onchainos token holders)

| # | Check | Threshold |
|---|---|---|
| H1 | Suspicious address holding | <= 30% |
| H2 | Phishing addresses | Block (`BLOCK_PHISHING = True`) |
| H3 | Suspicious address count | <= 10 |

---

## Momentum Score

```
Base Score (0-100):
  buyRatio × 40 + changePenalty × 20 + traderScore × 20 + liquidityScore × 20

Bonus (0-25):
  smartMoneyBuy +8 | top10<30% +5 | dsPaid +3 | communityTakeover +2
  sniper<5% +4 | devClean +3 | zeroSuspicious +2

Total = Base + min(Bonus, 25)
```

---

## 6-Layer Exit System

| Priority | Exit Type | Trigger Condition | Sell Ratio |
|--------|----------|------|----------|
| EXIT 0 | Ranking Exit | Dropped out of Top 20 and held >= 1min | 100% |
| EXIT 1 | Hard Stop Loss | PnL <= -15% | 100% |
| EXIT 2 | Quick Stop | Held >= 3min and PnL <= -8% | 100% |
| EXIT 3 | Trailing Stop | Peak PnL >= +10% then drawdown >= 8% | 100% |
| EXIT 4 | Time Stop | Held >= 2h | 100% |
| EXIT 5 | Tiered Take Profit | +8% sell 30% / +20% sell 35% / +40% sell 35% | Partial |

---

## Session Risk Control

| Rule | Value |
|---|---|
| Daily Loss Limit | `DAILY_LOSS_LIMIT = 0.15` (ratio of TOTAL_BUDGET, i.e., stop for the day after 15% loss) |
| Consecutive Loss Pause | 3 times → pause 15min (`MAX_CONSEC_LOSS = 3`, `PAUSE_CONSEC_SEC = 900`) |
| Cumulative Loss Stop | >= 0.10 SOL → stop trading (`SESSION_STOP_SOL = 0.10`) |
| Max Positions | `MAX_POSITIONS = 5` |
| Max Hold Time | `MAX_HOLD_HOURS = 2` |
| Cooldown | `COOLDOWN_MIN = 30` (30 minutes after selling before buying the same token again) |

Daily loss limit scales automatically with `TOTAL_BUDGET` (ratio fixed at 15%). Consecutive loss counter resets on a winning trade. Session risk control auto-resets on bot restart.

---

## Iron Rules (Must Not Be Violated)

1. RPC balance 0 ≠ token doesn't exist (Solana RPC has severe latency). Unconfirmed positions require zeroCount >= 10 AND elapsed > 180s before discarding.
2. Writing to positions.json **requires** holding `_state_lock`.
3. When `order_status()` returns TIMEOUT, **always** create an unconfirmed position.
4. Safety check API failure → Fail-Closed, **do not buy**.
5. Rank Exit (EXIT 0) has the **highest priority**.
6. Daily loss limit triggered → stop all buying for the day.
7. `GAS_RESERVE` 0.01 SOL is never spent on trades.

---

## onchainos CLI Command Reference

| # | Command | Purpose |
|---|---|---|
| 1 | `onchainos token trending --chain solana --sort-by 2 --time-frame 2` | Leaderboard Top 20 |
| 2 | `onchainos token advanced-info --chain solana --address <addr>` | Safety check data |
| 3 | `onchainos token holders --chain solana --address <addr> --tag-filter <tag>` | Holder risk |
| 4 | `onchainos market prices --tokens 501:<addr1>,501:<addr2>,...` | Batch prices |
| 5 | `onchainos swap quote --from <from> --to <to> --amount <amt> --chain solana` | Quote |
| 6 | `onchainos swap swap --from <from> --to <to> --amount <amt> --chain solana --wallet <addr> --slippage <pct>` | Trade |
| 7 | `onchainos wallet addresses --chain 501` | Solana address |
| 8 | `onchainos wallet balance --chain 501` | Balance |
| 9 | `onchainos wallet contract-call --chain 501 --to <router> --unsigned-tx <callData>` | TEE signing |
| 10 | `onchainos wallet order-status --order-id <orderId>` | Trade confirmation |

---

## Troubleshooting

| Issue | Solution |
|---|---|
| "FATAL: onchainos CLI not found" | Install onchainos and ensure it is in PATH |
| Dashboard won't open | Check if port 3244 is in use: `lsof -i:3244` |
| Bot has no trade signals | Leaderboard may have no new entries; wait for changes |
| Login expired | Re-run `onchainos wallet login <email>` |
| Live mode buy fails | Check SOL balance >= MIN_WALLET_BAL (0.06) |

---

## Glossary

| Term | Definition |
|------|------|
| **Ranking Exit** | Ranking Exit — automatically sell entire position when token drops out of the Top 20 gainers leaderboard; exit when momentum is lost |
| **Slot Guard** | 13 basic metric pre-checks based on leaderboard data, zero additional API calls |
| **Advanced Safety** | 9 deep safety checks using `onchainos token advanced-info` to obtain Dev/Bundler/LP data |
| **Holder Risk** | 3 holder risk checks using `onchainos token holders` to detect suspicious/phishing addresses |
| **Momentum Score** | Momentum score (0-125), calculated from buy ratio, price change, trader count, liquidity, and safety bonuses |
| **Quick Stop** | Quick Stop — triggers when position is held for N minutes and loss exceeds N% (both conditions must be met) |
| **Trailing Stop** | Trailing Stop — triggers sell when profit reaches activation threshold then pulls back beyond threshold from peak |
| **Unconfirmed Position** | Pending position created when trade confirmation times out; requires multiple balance checks before discarding |
| **Fail-Closed** | When safety check API fails, treat as unsafe and do not buy |
| **TEE** | Trusted Execution Environment — onchainos signing is performed inside a secure enclave |
| **Agentic Wallet** | onchainos managed wallet with private keys inside TEE, never leaving the secure environment |
| **DAILY_LOSS_LIMIT** | Daily loss ratio (of TOTAL_BUDGET); when triggered, all buying stops for the day |
| **MC / MCAP** | Market Cap — token total supply × current price, measuring token scale |
| **LP** | Liquidity Pool — token pair pool on DEX for trading; larger LP means lower slippage |
| **LP Burn** | Permanently burning LP tokens to ensure liquidity cannot be withdrawn by developers |
| **Rug Pull** | Malicious act where developers suddenly withdraw liquidity or dump all holdings, crashing the token price to zero |
| **Dev** | Token developer/deployer — in the Meme coin context, refers to the token contract creator; their holdings and history are important risk indicators |
| **Bundler** | Bundle trader — addresses that buy large amounts through bundled transactions at token launch; may be insiders or manipulators |
| **Sniper** | Sniper — bot addresses that auto-buy tokens instantly at launch; concentrated holdings may create sell pressure |
| **Honeypot** | Malicious token contract that can only be bought but not sold (or has extremely high sell tax) |
| **Slippage** | Difference between expected and actual execution price; worse liquidity means higher slippage |
| **lamports** | Smallest unit of SOL, 1 SOL = 1,000,000,000 lamports |
| **Native SOL** | SOL native token address `11111111111111111111111111111111` (32 ones), must be used as --from in swap |
| **WSOL** | Wrapped SOL (So11...112), SPL Token wrapped form of SOL, cannot be used as swap --from |
