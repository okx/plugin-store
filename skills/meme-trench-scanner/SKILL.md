---
name: meme-trench-scanner
description: >
  Meme Trench Scanner v1.0 — Agentic Wallet TEE signing automated trading bot.
  onchainos CLI driven (no API Key needed), full coverage of 11 Solana Launchpads,
  5m/15m B/S precision signal detection, price position filter (TOP_ZONE 85%),
  TP2 100% exit (no moon bag), TraderSoul AI observation system,
  FAST_DUMP 10-second crash detection, 3-check position protection.
  Triggers when the user mentions meme trench scanner, meme scanner, chain scanner,
  memepump scan, Tranches scan, pump.fun chain scan, safety filter chain scan,
  dev rug detection, bundler filter, on-chain scanning strategy, 扫链, Meme 扫链,
  or wants to automatically scan and trade pump.fun migrated tokens based on memepump.

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
REPORT_FLAG="$HOME/.plugin-store/reported/meme-trench-scanner"
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
    -d '{"name":"meme-trench-scanner","version":"1.0.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"meme-trench-scanner","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


# Meme Trench Scanner v1.0

> This strategy is a real trading bot. Make sure you understand the risks before use. It is recommended to test in Paper Mode first.

---

## Disclaimer

**This strategy script, parameter configuration, and all related documentation are for educational research and technical reference only, and do not constitute any form of investment advice, trading guidance, or financial recommendation.**

1. **Extreme Risk Warning**: Meme Trench Scanner targets newly launched small-cap Meme tokens, which represent **the highest-risk trading type** in cryptocurrency. Tokens may go to zero within minutes of launch (Rug Pull, Dev Dump, liquidity drain). You may lose your entire invested capital.
2. **Parameters for Reference Only**: All default parameters in this strategy (position size, take profit/stop loss, safety detection thresholds, scan frequency, etc.) are set based on general scenarios and **are not guaranteed to be suitable for any specific market environment**. Optimal parameters may vary greatly across different Launchpads and market cycles.
3. **User Customization**: Users are encouraged to deeply understand the meaning of each parameter and modify them according to their own strategy logic and risk preferences. Every parameter in `config.py` is annotated with comments for easy customization.
4. **No Guarantee of Profit**: Past performance does not represent future results. Even tokens that pass safety checks may still cause losses due to sudden market changes, contract vulnerabilities, etc.
5. **High-Frequency Trading Costs**: Accumulated fees, slippage, and gas costs from high-frequency chain scanning strategies may significantly erode profits. Please fully evaluate trading costs.
6. **Technical Risks**: On-chain transactions are irreversible. RPC node latency, network congestion, API rate limiting, and other technical factors may cause transaction failures or price deviations.
7. **Third-Party Dependency Risks**: This strategy depends on onchainos CLI, OKX API, and the Solana network among other third-party infrastructure. Their availability, accuracy, and stability are beyond the strategy author's control. Any changes, interruptions, or failures in these services may cause the strategy to malfunction or produce unexpected losses.
8. **Regulatory/Legal Risks**: Cryptocurrency trading may be subject to strict restrictions or prohibition in some countries and regions. Users should understand and ensure compliance with all applicable laws and regulations in their jurisdiction before using this strategy.
9. **Tax Risks**: Frequent trading may generate a large number of taxable events. Users should understand and comply with local tax laws regarding the reporting and payment of taxes on cryptocurrency trading gains.
10. **Assume All Responsibility**: This strategy is provided "AS-IS" without any express or implied warranties. All trading decisions made using this strategy and their consequences are the sole responsibility of the user. The strategy author, developers, distributors, and their affiliates are not liable for any direct, indirect, incidental, or special losses.

**Recommendation**: For first-time use, please run in Paper Mode (`PAPER_TRADE = True`) to fully familiarize yourself with the strategy logic and parameter behavior before considering whether to switch to Live Trading.

---

## File Structure

```
Meme Trench Scanner - Meme 扫链/
├── skill.md          ← This file (strategy documentation)
├── config.py         ← All adjustable parameters (modify parameters here only)
├── scan_live.py      ← Strategy main program
├── dashboard.html    ← Web Dashboard UI
├── scan_positions.json   ← [Auto-generated] Position data
├── scan_trades.json      ← [Auto-generated] Trade history
├── trader_soul.json      ← [Auto-generated] TraderSoul personality data
└── scan_recently_closed.json ← [Auto-generated] Cooldown records
```

---

## Prerequisites

### 1. Install onchainos CLI (>= 2.1.0)

```bash
# Check if already installed
onchainos --version

# If not installed, follow the onchainos official documentation
# Ensure onchainos is in PATH or located at ~/.local/bin/onchainos
```

### 2. Log in to Agentic Wallet (TEE Signing)

```bash
# One-time login (email verification)
onchainos wallet login <your-email>

# Verify login status
onchainos wallet status
# → loggedIn: true

# Confirm Solana address
onchainos wallet addresses --chain 501
```

> Agentic Wallet uses TEE secure enclave signing; private keys are never exposed to code/logs/network.
> No need to set WALLET_PRIVATE_KEY environment variable.

### 3. No pip install needed

This strategy only depends on Python standard library + onchainos CLI, no third-party packages required.

---

## AI Agent Startup Interaction Protocol

> **When the user requests to start this strategy, the AI Agent must follow the procedure below and must not skip directly to startup.**

### Phase 1: Display Strategy Overview

Show the user the following content:

```
🔍 Meme Trench Scanner v1.0 — Solana Meme Automated Trading Bot

This strategy scans newly launched tokens from 11 Solana Launchpads
(pump.fun, Believe, LetsBonk, etc.) using TX acceleration + volume surge
+ B/S ratio triple signal detection, and automatically executes buys
and take profit/stop loss.

🧪 Current: Paper Mode — no real money spent, observe signals

⚠️ Risk Notice: Meme tokens carry extremely high risk. You may lose your entire invested capital.

Default parameters (for reference only, recommend adjusting to your situation):
  Position size:   MINIMUM 0.15 SOL / STRONG 0.25 SOL
  Max exposure:    1.00 SOL
  Max positions:   7
  Take profit:     TP1 +15% / TP2 +25%
  Stop loss:       -15% ~ -20% (auto-adjusted by market heat)
  Trailing stop:   5% drawdown after TP1 hit → exit
  Max hold time:   30 minutes

All parameters can be freely modified in config.py to suit your trading style.
```

### Q1: Risk Preference (Required)

- 🛡️ Conservative: Quick in-and-out, small take profit, strict stop loss
- ⚖️ Default: Balanced configuration (recommended)
- 🔥 Aggressive: Large take profit, wide stop loss

→ Parameter mapping (for AI Agent to write to config.py, no need to show to user):

| Preference | TP1_PCT | TP2_PCT | S1_PCT (SCALP/hot/quiet) | MAX_HOLD_MIN | MAX_POSITIONS | TRAILING_DROP |
|------|---------|---------|--------------------------|--------------|---------------|---------------|
| Conservative | 0.10 | 0.18 | -0.12 / -0.15 / -0.15 | 20 | 5 | 0.03 |
| Default | 0.15 | 0.25 | -0.15 / -0.20 / -0.20 | 30 | 7 | 0.05 |
| Aggressive | 0.25 | 0.40 | -0.25 / -0.30 / -0.30 | 45 | 10 | 0.08 |

> Note: S1_PCT is automatically split into three tiers by market heat (SCALP=rapid/hot=active/quiet=calm), no user selection needed.

### Q2: Switch to Live Trading?

- A. 🧪 Stay in Paper Mode, start directly (recommended default)
- B. 💰 Switch to Live Trading mode

**Choose A** → Proceed directly to startup steps.

**Choose B** → Enter Live Trading sub-flow:

1. ⚠️ Confirm with user:
   "Live Trading will use real SOL. Losses are irreversible. Confirm switch to Live Trading?"
   - User confirms → Continue
   - User declines → Fall back to Paper Mode

2. Ask for max exposure in SOL (default 1.00 SOL)

3. AI auto-calculates (let M = user's input exposure):
   - `MAX_SOL = M`
   - `SOL_PER_TRADE`:
     - `SCALP: max(M × 0.25, 0.05)` [disabled in current version]
     - `MINIMUM: max(M × 0.15, 0.05)`
     - `STRONG: max(M × 0.25, 0.05)`
   - `PAUSE_LOSS_SOL = M × 0.30` (cumulative loss pause line)
   - `STOP_LOSS_SOL = M × 0.50` (cumulative loss stop line)

4. Show calculation results to user and confirm:
   "Your Live Trading config: Max exposure X SOL, per-trade MINIMUM/STRONG = Y/Y SOL, loss pause Z SOL / stop W SOL. Confirm?"
   - User confirms → Write to config.py
   - User requests adjustment → Return to step 2

5. Set mode parameters:
   - `PAPER_TRADE = False`
   - `PAUSED = False`

### Startup

1. Modify corresponding parameters in `config.py` based on user responses
2. Set `PAUSED = False` (allow bot to run normally after interactive confirmation)
3. Check prerequisites: `onchainos --version`, `onchainos wallet status`
4. Start bot: `python3 scan_live.py`
5. Show Dashboard link: `http://localhost:3241`
6. Inform user: Currently in Paper Mode. To switch to Live Trading, modify `PAPER_TRADE = False` in `config.py`

If the user says "use default config" or "just run it", only set `PAUSED = False`, leave everything else unchanged, and start directly in Paper Mode.

### Special Cases

- User explicitly says "don't ask me, just run" → Start with default parameters (Paper Mode), but must show Phase 1 overview + set `PAUSED = False`
- User is a returning user (configuration history exists in conversation) → Remind of previous configuration, ask whether to reuse

---

## Quick Start

> ⚠️ Before starting, confirm the `PAPER_TRADE` value in config.py — `True` for Paper Trading, `False` for Live Trading.

```bash
cd ~/CC/Meme\ Trench\ Scanner\ -\ Meme\ 扫链

# 1. Confirm onchainos is logged in
onchainos wallet status

# 2. Start bot (foreground, Ctrl+C to stop)
python3 scan_live.py

# Or run in background
nohup python3 scan_live.py > bot.log 2>&1 &

# 3. Open Dashboard
open http://localhost:3241

# 4. Stop
pkill -f scan_live.py
```

> **First startup defaults to PAUSED=True, will not open new positions. After confirming everything is normal, modify config.py PAUSED=False.**

---

## Parameter Adjustment

**All adjustable parameters are in `config.py`**, no need to modify `scan_live.py`.

### Common Adjustments

| Need | Modify in `config.py` |
|---|---|
| Pause/resume trading | `PAUSED = True/False` |
| Adjust position size | `SOL_PER_TRADE = {"SCALP": 0.25, "MINIMUM": 0.15, "STRONG": 0.25}` |
| Adjust max exposure | `MAX_SOL = 1.00` |
| Adjust max positions | `MAX_POSITIONS = 7` |
| Adjust take profit | `TP1_PCT = 0.15` (15%), `TP2_PCT = 0.25` (25%) |
| Adjust stop loss | `S1_PCT = {"SCALP": -0.15, "hot": -0.20, "quiet": -0.20}` |
| Adjust scan speed | `LOOP_SEC = 10` (seconds) |
| MC range | `MC_MIN = 50_000`, `MC_CAP = 800_000` |
| Paper Trading | `PAPER_TRADE = True` |
| Limit total trades | `MAX_TRADES = 50` (0=unlimited) |
| Dashboard port | `DASHBOARD_PORT = 3241` |

Restart bot for changes to take effect.

> config.py also contains more advanced parameters (Launchpad protocol IDs, trade blacklist, Pullback Watchlist, LP Lock details, NEW stage filters, etc.). See comments in config.py for details.

---

## Strategy Architecture

```
scan_live.py (single-file Bot)
├── onchainos CLI (data + execution + safety — no API Key)
├── scanner_loop()     ← background thread, every 10s
│   ├── memepump_token_list()   Token discovery (11 Launchpads)
│   ├── pre_filter()            Basic filters (MC/Age/B&S/Vol/Holders)
│   ├── hot_mode_check()        Market heat detection
│   └── detect_signal()         Signal detection
│       ├── 5m/15m B/S (raw trades calculation)
│       ├── TX acceleration detection (Signal A)
│       ├── Volume surge (Signal B)
│       ├── Anti-chase protection
│       ├── TOP_ZONE 85% filter
│       ├── Confidence scoring
│       └── → try_open_position() (async thread)
│           └── deep_safety_check() (Dev+Bundle+LP+Aped)
├── monitor_loop()     ← background thread, every 1s
│   ├── _quick_wallet_sync()    Wallet sync
│   ├── check_position()        Exit decision
│   │   ├── HE1: -50% emergency exit
│   │   ├── FAST_DUMP: -15% within 10s
│   │   ├── S1: Stop loss / Breakeven
│   │   ├── S3: Time stop
│   │   ├── Trailing: 5% drawdown after TP1
│   │   ├── TP1: +15% partial sell
│   │   └── TP2: +25% full exit
│   └── wallet_audit()          Periodic reconciliation
├── TraderSoul                  AI personality (observe only, no param changes)
├── Dashboard (port 3241)       Web UI
└── Persistent files (JSON, atomic write)
```

---

## Signal Tiers

| Tier | Conditions | Position |
|---|---|---|
| **SCALP** | sig_a + sig_c | 0.25 SOL (currently disabled) |
| **MINIMUM** | sig_a + sig_c (no sig_b) | 0.15 SOL |
| **STRONG** | sig_a + sig_b + sig_c | 0.25 SOL |

In the current version, SCALP signals are skipped; only MINIMUM and STRONG execute trades.

---

## Safety Detection

### Server-Side Filtering (memepump tokens parameters)

| Check | Threshold |
|---|---|
| MC range | $50K - $800K |
| Holders | >= 50 |
| Bundler holdings | <= 15% |
| Dev holdings | <= 10% |
| Insider | <= 15% |
| Sniper | <= 20% |
| Top 10 holdings | <= 40% |
| Fresh wallets | <= 40% |

### Deep Safety (deep_safety_check)

| Check | Threshold |
|---|---|
| Dev rug count | = 0 (zero tolerance) |
| Dev rug rate | <= 50% |
| Dev holdings | <= 10% |
| Dev historical launches | <= 800 |
| Bundler ATH | <= 25% |
| Bundler count | <= 30 |
| Aped wallets | <= 10 |
| LP Lock | >= 80% |
| Serial Rugger | death rate <= 60% |

---

## 7-Layer Exit System

| Priority | Exit Type | Trigger Condition | Sell Ratio |
|--------|---------|---------|---------|
| **HE1** | Emergency exit | PnL <= -50% | 100% |
| **FAST_DUMP** | Crash detection | >= 15% drop within 10s | 100% |
| **S1** | Stop loss | PnL <= -15%~-20% (by market heat) | 100% |
| **S3** | Time stop | SCALP 5min / hot 8min / quiet 15min still losing | 100% |
| **Trailing** | Trailing stop | >= 5% drawdown from peak after TP1 hit | 100% |
| **TP1** | First take profit | +15% | 40-50% |
| **TP2** | Second take profit | +25% | 100% |

> Priority is top to bottom; once triggered, executes immediately without checking subsequent layers.

---

## Session Risk Control

| Rule | Value |
|---|---|
| Consecutive loss pause | 2 losses → pause 15min |
| Cumulative loss pause | >= 0.30 SOL → pause 30min |
| Cumulative loss stop | >= 0.50 SOL → stop trading |
| Max hold time | 30min |
| HKT sleep | 04:00-08:00 no new positions |
| MAX_TRADES | Auto-stop after 50 trades |

---

## Iron Rules (Must Not Be Violated)

1. **NEVER** delete a position based on a single balance check. Must have `zero_balance_count >= 3`.
2. **NEVER** call `save_positions()` outside of `pos_lock`.
3. When `tx_status()` returns TIMEOUT, **always** create an `unconfirmed=True` position.
4. RPC balance 0 ≠ token does not exist (Solana RPC has significant latency).

---

## onchainos CLI Command Reference

| # | Command | Purpose |
|---|---|---|
| 1 | `onchainos memepump tokens --chain solana --stage MIGRATED ...` | Token discovery |
| 2 | `onchainos memepump token-details --chain solana --address <addr>` | Token details |
| 3 | `onchainos memepump token-dev-info --chain solana --address <addr>` | Dev safety |
| 4 | `onchainos memepump token-bundle-info --chain solana --address <addr>` | Bundler |
| 5 | `onchainos memepump aped-wallet --chain solana --address <addr>` | Aped wallets |
| 6 | `onchainos memepump similar-tokens --chain solana --address <addr>` | Similar tokens |
| 7 | `onchainos token price-info --chain solana --address <addr>` | Real-time price |
| 8 | `onchainos market kline --chain solana --address <addr> --bar 1m` | K-line |
| 9 | `onchainos token trades --chain solana --address <addr>` | Trade history |
| 10 | `onchainos swap quote --chain solana --from <> --to <> --amount <>` | Quote |
| 11 | `onchainos swap swap --chain solana --from <> --to <> --amount <> --slippage <> --wallet <>` | Build transaction |
| 12 | `onchainos wallet contract-call --chain 501 --to <> --unsigned-tx <>` | TEE sign + broadcast |
| 13 | `onchainos wallet history --tx-hash <> --chain-index 501` | Transaction confirmation |
| 14 | `onchainos wallet status` | Login status |
| 15 | `onchainos wallet addresses --chain 501` | Solana address |
| 16 | `onchainos portfolio all-balances --address <> --chains solana` | All balances |
| 17 | `onchainos portfolio token-balances --address <> --tokens 501:<mint>` | Single token balance |

---

## Troubleshooting

| Problem | Solution |
|---|---|
| "FATAL: onchainos CLI not found" | Install onchainos and ensure it is on PATH |
| "FATAL: Agentic Wallet not logged in" | Run `onchainos wallet login <email>` |
| "FATAL: Unable to parse Solana address" | Check `onchainos wallet addresses --chain 501` |
| Dashboard won't open | Check if port 3241 is in use: `lsof -i:3241` |
| Bot not trading | Check config.py `PAUSED = True`, change to `False` |
| Transaction failed InstructionError | swap --from must use `11111111111111111111111111111111` (native SOL) |
| Login expired | Re-run `onchainos wallet login <email>` |

---

## Glossary

| Term | Definition |
|------|------|
| **SCALP / hot / quiet** | Three market heat tiers — SCALP=rapid, hot=active, quiet=calm; auto-detected, affects stop loss and position size |
| **Signal A (TX Acceleration)** | Transaction frequency surge detection — triggers when current txs/min exceeds baseline x threshold |
| **Signal B (Volume Surge)** | 5m/15m volume breakout detection |
| **Signal C (B/S Ratio)** | Buy/sell ratio confirmation — buy count / sell count > threshold |
| **Confidence** | Signal confidence score (0-100), calculated from Signal A/B/C combined |
| **TOP_ZONE** | Price position filter — current price's position within historical range, >85% means near ATH, skip |
| **FAST_DUMP** | 10-second crash detection — 15% drop within 10s triggers emergency exit |
| **deep_safety_check** | Deep safety check — Dev rug history, Bundler holdings, LP Lock, Aped wallets, etc. |
| **Trailing Stop** | Trailing stop — after TP1 hit, full exit when drawdown from peak exceeds threshold |
| **3-check Position Protection** | Balance check protection — requires 3 consecutive zero-balance readings before deleting position, prevents RPC false positives |
| **Fail-Closed** | When safety check API fails, treat as unsafe and do not buy |
| **TEE** | Trusted Execution Environment — onchainos signing is performed within a secure enclave |
| **Agentic Wallet** | onchainos managed wallet, private key stays inside TEE, never leaves the secure environment |
| **HKT Sleep** | No new positions during 04:00-08:00 Hong Kong Time, avoiding low-liquidity period |
| **memepump** | OKX Launchpad token aggregation API, covering 11 Solana Launchpads |
| **TraderSoul** | AI observation system — records trading behavior, personality tags, and cumulative performance; observe only, never modifies parameters; data saved in trader_soul.json |
| **Launchpad** | Token launch platform — pump.fun, Believe, LetsBonk, etc.; new tokens debut here and establish initial liquidity |
| **MC / MCAP** | Market Cap — token total supply x current price, measures token scale |
| **LP** | Liquidity Pool — token pair liquidity pool on DEX; larger LP means lower buy/sell slippage |
| **LP Lock** | Locking LP tokens for a period to ensure liquidity cannot be pulled by developers in the short term |
| **Rug Pull** | Malicious act where developers suddenly withdraw liquidity or dump all holdings, causing token price to go to zero |
| **Dev** | Token developer/deployer — in the Meme token context, refers to the creator of the token contract; their holdings and historical behavior are important risk indicators |
| **Bundler** | Bundle trader — addresses that buy large amounts through bundled transactions at token launch; may be insiders or manipulators |
| **Sniper** | Sniper — bot addresses that automatically buy at the instant of token launch; concentrated holdings may create sell pressure |
| **Aped Wallet** | Wallets that bought large amounts early in a token's life; too many indicates the token is being targeted by bots |
| **Honeypot** | Malicious token contract where you can buy but cannot sell (or sell tax is extremely high) |
| **Slippage** | Difference between expected and actual execution price; worse liquidity means higher slippage |
| **lamports** | Smallest unit of SOL, 1 SOL = 1,000,000,000 lamports |
| **Native SOL** | SOL native token address `11111111111111111111111111111111` (32 ones), must use this address for swap --from |
| **WSOL** | Wrapped SOL (So11...112), SPL Token wrapped form of SOL, cannot be used for swap --from |
