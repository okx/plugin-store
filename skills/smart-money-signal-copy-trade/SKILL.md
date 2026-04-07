---
name: smart-money-signal-copy-trade
description: >
  Smart Money Signal Copy Trade v1.0 — onchainos Agentic Wallet + Cost-Aware TP + Dev/Bundler Safety + Session Risk Control.
  Triggers when the user mentions smart money strategy, signal strategy, copy trading,
  whale tracking, KOL copy trading, on-chain signal trading, co-riding addresses,
  take profit / stop loss, risk preference,
  or wants to automatically buy/sell based on smart money signals.
  Runtime file: bot.py (includes Web Dashboard http://localhost:3248)
  Config file: config.py (hot-reload)
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
REPORT_FLAG="$HOME/.plugin-store/reported/smart-money-signal-copy-trade"
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
    -d '{"name":"smart-money-signal-copy-trade","version":"1.0.0"}' >/dev/null 2>&1 || true
  # Report to OKX API (with HMAC-signed device token)
  curl -s -X POST "https://www.okx.com/priapi/v1/wallet/plugins/download/report" \
    -H "Content-Type: application/json" \
    -d '{"pluginName":"smart-money-signal-copy-trade","divId":"'"$DIV_ID"'"}' >/dev/null 2>&1 || true
  touch "$REPORT_FLAG"
fi
```

---


# Smart Money Signal Copy Trade v1.0

> This is a real trading bot. Make sure you understand the risks before use. It is recommended to test in Paper Mode first.

---

## Disclaimer

**This strategy script, parameter configuration, and all related documentation are provided solely for educational research and technical reference purposes. They do not constitute any form of investment advice, trading guidance, or financial recommendation.**

1. **High Risk Warning**: Cryptocurrency trading (especially on-chain meme tokens) carries extremely high risk. Prices may fluctuate drastically or drop to zero within seconds. You may lose all invested capital.
2. **Signals Are Not Certainties**: Smart Money / KOL / Whale buy signals only reflect on-chain behavior at a specific moment and **do not guarantee the token will appreciate**. Signal sources may have delays, misjudgments, or manipulation. Smart money address labels come from third-party data sources, and their accuracy cannot be guaranteed.
3. **Parameters Are For Reference Only**: All default parameters in this strategy (take profit, stop loss, position size, safety thresholds, etc.) are set for general scenarios and **are not guaranteed to be suitable for any specific market environment**. Users should adjust all parameters based on their own risk tolerance, trading experience, and market judgment.
4. **User Customization Encouraged**: Users are encouraged to deeply understand the meaning of each parameter and modify them according to their own strategy logic and risk preferences. Every parameter in `config.py` is annotated with comments for easy customization.
5. **No Profit Guarantee**: Past performance does not represent future results. Even parameters that perform well in backtesting may fail in live trading due to changing market conditions.
6. **Technical Risk**: On-chain transactions are irreversible. Smart contracts may contain vulnerabilities. Network congestion may cause transaction delays or failures.
7. **Third-Party Dependency Risk**: This strategy depends on onchainos CLI, OKX API, and the Solana network among other third-party infrastructure. Their availability, accuracy, and stability are beyond the strategy author's control. Any changes, interruptions, or failures in these services may cause the strategy to malfunction or produce unexpected losses.
8. **Regulatory/Legal Risk**: Cryptocurrency trading may be strictly restricted or prohibited in certain countries and jurisdictions. Users should independently verify and ensure compliance with all applicable laws and regulations in their jurisdiction before using this strategy.
9. **Tax Risk**: Frequent trading may generate numerous taxable events. Users should independently understand and comply with local tax laws regarding cryptocurrency trading gains reporting and payment requirements.
10. **Assumption of Responsibility**: This strategy is provided "AS-IS" without any express or implied warranties. All trading decisions made using this strategy and their consequences are the sole responsibility of the user. The strategy authors, developers, distributors, and their affiliates are not liable for any direct, indirect, incidental, or special losses.

**Recommendation**: For first-time use, run in Paper Mode (`DRY_RUN = True`). After thoroughly familiarizing yourself with the strategy logic and parameter behavior, consider whether to switch to Live Mode.

---

## File Structure

```
Smart Money Signal Copy Trade - 聪明钱信号跟单/
├── skill.md              ← This file (strategy documentation)
├── config.py             ← All adjustable parameters (modify only here, hot-reload)
├── bot.py                ← Main strategy program
├── dashboard.html        ← Web Dashboard UI
├── collision_guard.py    ← Cross-strategy collision detection
├── positions.json        ← [Auto-generated] Position data
└── signal_trades.json    ← [Auto-generated] Trade history
```

---

## Prerequisites

### 1. Install onchainos CLI (>= 2.0.0-beta)

```bash
# Check if already installed
onchainos --version

# If not installed, follow onchainos official documentation
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

> Agentic Wallet uses TEE secure enclave signing. Private keys are never exposed to code/logs/network.
> No need to set the WALLET_PRIVATE_KEY environment variable.

### 3. No pip install Required

This strategy only depends on the Python standard library + onchainos CLI. No third-party packages are needed.

---

## AI Agent Startup Interaction Protocol

> **When a user requests to start this strategy, the AI Agent must follow the procedure below. Do not skip directly to launch.**

### Phase 1: Present Strategy Overview

Show the user the following:

```
📡 Smart Money Signal Copy Trade v1.0 — Smart Money Signal Tracker

This strategy polls OKX Smart Money / KOL / Whale buy signals every 20 seconds.
When ≥3 smart wallets simultaneously buy the same token, it auto-buys after passing 15 safety filters.
A 7-layer exit system manages take profit and stop loss (Cost-Aware TP + Trailing + Time-Decay SL).

🧪 Current: Paper Mode — no real money spent, just observing signals

⚠️ Risk Notice: On-chain trading is high risk. You may lose all invested capital.

Default parameters (for reference only, adjust based on your situation):
  Position size:    high 0.020 / mid 0.015 / low 0.010 SOL
  Max positions:    6
  Min co-riders:    3 smart wallets
  Safety thresholds: MC≥$200K, Liq≥$80K, Holders≥300, Dev Rug=0
  Take Profit:      TP1 +5% / TP2 +15% / TP3 +30% (NET, cost-aware)
  Stop Loss:        -10% hard stop loss, tightens with time decay
  Trailing Stop:    Activates at +12% profit, exits on 10% drawdown
  Max hold time:    4 hours

All parameters can be freely modified in config.py to suit your trading style.
```

### Q1: Risk Preference (Mandatory)

- 🛡️ Conservative: Small positions, tight stop loss, fewer positions
- ⚖️ Default: Balanced configuration (recommended)
- 🔥 Aggressive: Large positions, wide stop loss, more positions

→ Parameter mapping (for AI Agent to write to config.py, not shown to user):

| Preference | SL_MULTIPLIER | MAX_POSITIONS | MIN_WALLET_COUNT | TIME_STOP_MAX_HOLD_HRS | TRAIL_ACTIVATE | TRAIL_DISTANCE |
|------|---------------|---------------|------------------|------------------------|----------------|----------------|
| Conservative | 0.92 (-8%) | 4 | 5 | 2 | 0.08 | 0.06 |
| Default | 0.90 (-10%) | 6 | 3 | 4 | 0.12 | 0.10 |
| Aggressive | 0.85 (-15%) | 8 | 3 | 6 | 0.18 | 0.12 |

### Q2: Switch to Live Trading?

- A. 🧪 Stay in Paper Mode, launch directly (recommended default)
- B. 💰 Switch to Live Mode

**Option A** → Proceed directly to launch steps.

**Option B** → Enter Live Mode sub-flow:

1. ⚠️ Confirm with user:
   "Live Mode will use real SOL for trading. Losses are irreversible. Confirm switch to Live Mode?"
   - User confirms → Continue
   - User declines → Fall back to Paper Mode

2. Ask for total budget in SOL (range 0.5-10, default 1.0 SOL)

3. AI auto-calculates (let B = user's budget input):
   - `SESSION_STOP_SOL = B × 0.10` (stop at 10% loss)
   - `SESSION_LOSS_LIMIT_SOL = B × 0.05` (pause at 5% loss)
   - `POSITION_TIERS`:
     - `high: {"min_addr": 8, "sol": max(0.020 × B, 0.005)}`
     - `mid:  {"min_addr": 5, "sol": max(0.015 × B, 0.005)}`
     - `low:  {"min_addr": 3, "sol": max(0.010 × B, 0.005)}`

4. Show calculated results to user and confirm:
   "Your Live Mode config: Total budget X SOL, per-trade high/mid/low = Y/Y/Y SOL, loss stop limit Z SOL. Confirm?"
   - User confirms → Write to config.py
   - User requests adjustments → Return to step 2

5. Set mode parameters:
   - `DRY_RUN = False`
   - `PAUSED = False`

### Launch

1. Modify corresponding parameters in `config.py` based on user responses
2. Set `PAUSED = False` (allow bot to run normally after interaction confirmation)
3. Check prerequisites: `onchainos --version`, `onchainos wallet status`
4. Start bot: `python3 bot.py`
5. Show Dashboard link: `http://localhost:3248`
6. Inform user: Currently in Paper Mode. To switch to Live Mode, change `DRY_RUN = False` in `config.py`

If user says "use default config" or "just run it", only set `PAUSED = False`, leave everything else unchanged, and launch in Paper Mode.

### Special Cases

- User explicitly says "don't ask me, just run" → Launch with default parameters (Paper Mode), but must show Phase 1 overview + set `PAUSED = False`
- User is a returning user (config history exists in conversation) → Remind of previous config, ask whether to reuse

---

## Quick Start

> ⚠️ Before launching, confirm the `DRY_RUN` value in config.py — `True` for Paper, `False` for Live.

```bash
cd ~/CC/Smart\ Money\ Signal\ Copy\ Trade\ -\ 聪明钱信号跟单

# 1. Confirm onchainos is logged in
onchainos wallet status

# 2. Start bot (foreground, Ctrl+C to stop)
python3 bot.py

# 3. Open Dashboard
open http://localhost:3248

# 4. Stop
pkill -f bot.py
```

> **First launch defaults to PAUSED=True, no new positions will be opened. After confirming everything is normal, change config.py PAUSED=False.**
> config.py supports hot-reload (`importlib.reload`). No bot restart needed after changes.

---

## Parameter Tuning

**All adjustable parameters are in `config.py`** — no need to modify `bot.py`.

### Common Adjustments

| Need | Modify in `config.py` |
|---|---|
| Pause/Resume trading | `PAUSED = True/False` |
| Paper/Live Mode switch | `DRY_RUN = True/False` |
| Adjust position size | `sol` values in `POSITION_TIERS` for each tier |
| Adjust max positions | `MAX_POSITIONS = 6` |
| Adjust min co-rider count | `MIN_WALLET_COUNT = 3` |
| Adjust take profit | `pct` and `sell` in `TP_TIERS` |
| Adjust hard stop loss | `SL_MULTIPLIER = 0.90` (-10%) |
| Adjust time-decay SL | `TIME_DECAY_SL` list |
| Adjust trailing stop | `TRAIL_ACTIVATE = 0.12`, `TRAIL_DISTANCE = 0.10` |
| Adjust max hold time | `TIME_STOP_MAX_HOLD_HRS = 4` |
| MC range | `MIN_MCAP = 200_000`, `MIN_LIQUIDITY = 80_000` |
| Session loss limits | `SESSION_STOP_SOL = 0.10`, `SESSION_LOSS_LIMIT_SOL = 0.05` |
| Scan interval | `POLL_INTERVAL_SEC = 20` |
| Dashboard port | `DASHBOARD_PORT = 3248` |

Changes take effect automatically via hot-reload (no bot restart needed).

---

## Strategy Architecture

```
bot.py (Single-file Bot)
├── onchainos CLI (Data + Execution + Safety — no API Key)
├── run()                  ← Main loop, every 20s
│   ├── signal list()              Smart Money/KOL/Whale signals
│   ├── Level 1 Pre-filter         soldRatio < 80%, walletCount >= 3
│   └── open_position()            15 deep verifications → Buy
│       ├── market prices           MC/Liq/Holders/Price
│       ├── token search            Community verification status
│       ├── market candles          K1 pump < 15%
│       ├── token advanced-info     Dev rug/Bundler/LP/Top10
│       ├── wallet balance          SOL balance check
│       ├── swap quote              Honeypot detection + quote
│       └── → execute_swap()        Paper: quote / Live: swap + TEE signing
├── monitor_positions()    ← Background thread, every 20s
│   ├── market prices               Batch prices
│   └── check_position()            7-layer exit decision
│       ├── EXIT 0: Liquidity emergency exit (liq < $5K)
│       ├── EXIT 1: Dust cleanup (< $0.10)
│       ├── EXIT 2: Hard stop loss (-10%)
│       ├── EXIT 3: Time-decay SL (30min/-8%, 60min/-5%)
│       ├── EXIT 4: Tiered take profit TP1/TP2/TP3 (cost-aware)
│       ├── EXIT 5: Trailing stop (+12% activate, 10% drawdown)
│       ├── EXIT 6: Trend time stop (15m candle reversal)
│       └── EXIT 7: Hard time stop (4h)
├── Session Risk Control     Consecutive loss pause / Cumulative loss stop
├── Dashboard (port 3248)    Web UI
└── Persistence files (JSON, atomic write)
```

**Scheduled Tasks:**

| Task | Frequency | Responsibility |
|------|------|------|
| `run()` | Every 20s | Poll signals → Pre-filter → Deep verification → Buy |
| `monitor_positions()` | Every 20s | Fetch position prices → 7-layer exit system |
| `importlib.reload(config)` | Auto each cycle | Hot-reload config.py parameters |

---

## Safety Checks

### Level 1 Pre-filter (2 checks, based on signal list data, 0 extra API calls)

| # | Check | Threshold |
|---|---|---|
| 1 | Smart Money sell ratio | `soldRatioPercent` < 80% |
| 2 | Co-rider wallet count | `triggerWalletCount` >= 3 |

### Level 2 Deep Verification (13 checks, via onchainos CLI)

| # | Check | Threshold | Data Source |
|---|---|---|---|
| 1 | Market Cap | >= $200,000 | `market prices` |
| 2 | Liquidity | >= $80,000 | `market prices` |
| 3 | Holder count | >= 300 | `market prices` |
| 4 | Liquidity/MC ratio | >= 5% | `market prices` |
| 5 | Top10 concentration | <= 50% | `token advanced-info` |
| 6 | Holder density | >= 50 per million MC | `market prices` |
| 7 | K1 price change | <= 15% | `market candles` |
| 8 | Dev rug history | = 0 (zero tolerance) | `token advanced-info` |
| 9 | Dev farm | <= 20 | `token advanced-info` |
| 10 | Dev holding | <= 15% | `token advanced-info` |
| 11 | Bundler ATH | <= 25% | `token advanced-info` |
| 12 | Bundler count | <= 5 | `token advanced-info` |
| 13 | LP burn | >= 80% | `token advanced-info` |

### Final Pre-Buy Check

- SOL balance >= position size + `SOL_GAS_RESERVE` (0.05)
- Honeypot detection: `isHoneyPot == false && taxRate <= 5` (via `swap quote`)
- Position count < `MAX_POSITIONS`
- Not in cooldown / Session not paused/stopped

---

## Position Tiers

| Tier | Condition | Position |
|---|---|---|
| **high** | >= 8 co-rider wallets | 0.020 SOL |
| **mid** | >= 5 co-rider wallets | 0.015 SOL |
| **low** | >= 3 co-rider wallets | 0.010 SOL |

**Cost Model (Breakeven by tier):**

| Tier | Fixed Cost Ratio | Slippage Cost (x2 legs) | Breakeven |
|---|---|---|---|
| high (0.020) | 0.001/0.020 = 5.0% | 2.0% | **7.0%** |
| mid (0.015) | 0.001/0.015 = 6.7% | 2.0% | **8.7%** |
| low (0.010) | 0.001/0.010 = 10.0% | 2.0% | **12.0%** |

> TP is **cost-aware** — `tp_threshold = net_target + breakeven_pct`. For the low tier, TP1 actually requires +17% raw price change to trigger (5% + 12% breakeven).

---

## 7-Layer Exit System

| Priority | Exit Type | Trigger Condition | Sell Ratio |
|--------|---------|---------|---------|
| **EXIT 0** | Liquidity emergency exit | `liq < $5,000` | 100% |
| **EXIT 1** | Dust cleanup | Position value < $0.10 | 100% |
| **EXIT 2** | Hard stop loss | `price <= entry × 0.90` (-10%) | 100% |
| **EXIT 3** | Time-decay SL | 30min: -8%, 60min: -5% | 100% |
| **EXIT 4** | Tiered take profit | TP1 +5% NET sell 30% / TP2 +15% sell 40% / TP3 +30% sell 100% | Partial |
| **EXIT 5** | Trailing stop | Peak PnL >= +12%, then drawdown >= 10% from peak | 100% |
| **EXIT 6** | Trend time stop | Position >= 30min and 15m candle reversal confirmed | 100% |
| **EXIT 7** | Hard time stop | Hold time >= 4h | 100% |

---

## Session Risk Control

| Rule | Value |
|---|---|
| Consecutive loss pause | 3 times → Pause 10min (`MAX_CONSEC_LOSS = 3`) |
| Cumulative loss pause | >= 0.05 SOL → Pause 30min (`SESSION_LOSS_LIMIT_SOL = 0.05`) |
| Cumulative loss stop | >= 0.10 SOL → Stop trading (`SESSION_STOP_SOL = 0.10`) |
| Max hold time | 4h (`TIME_STOP_MAX_HOLD_HRS = 4`) |

Consecutive loss counter resets on a profitable trade. Session risk control auto-resets on bot restart.

---

## Iron Rules (Must Not Be Violated)

1. `advanced-info` safety check failure → Fail-Closed, **do not buy**.
2. `soldRatioPercent > 80%` skip — Smart money is already selling, don't catch the falling knife.
3. K1 price change > 15% skip — Don't chase pumps.
4. Zero tolerance for Dev rug history — If there's a rug, don't buy.
5. After selling a token, set cooldown. **No re-buying** during cooldown period.
6. Position size is fixed per tier. **No adding to positions**.
7. If all three levels of buy price fallback fail (price <= 0), **do not open position**.
8. Daily loss limit reached → Stop all buying.
9. `SOL_GAS_RESERVE` 0.05 SOL is never spent on trades.
10. **Must** hold `state_lock` before writing to positions.json.

---

## onchainos CLI Command Reference

| # | Command | Purpose |
|---|---|---|
| 1 | `onchainos signal list --chain solana --wallet-type 1,2,3 --min-address-count 3` | Smart Money signals |
| 2 | `onchainos market prices --tokens 501:<addr1>,501:<addr2>,...` | Batch price/MC/Liq |
| 3 | `onchainos market candles --chain solana --address <addr> --bar 1m` | Candles (K1 pump detection) |
| 4 | `onchainos token search --chain solana --query <symbol>` | Community verification |
| 5 | `onchainos token advanced-info --chain solana --address <addr>` | Dev/Bundler/LP/Top10 |
| 6 | `onchainos swap quote --from 1111...1111 --to <token> --amount <lamports> --chain solana` | Quote + Honeypot detection |
| 7 | `onchainos swap swap --from <from> --to <to> --amount <amt> --chain solana --wallet <addr> --slippage <pct>` | Trade execution |
| 8 | `onchainos wallet addresses --chain 501` | Solana address |
| 9 | `onchainos wallet balance --chain 501` | SOL balance |
| 10 | `onchainos wallet contract-call --chain 501 --to <tx.to> --unsigned-tx <base58>` | TEE signing + broadcast |
| 11 | `onchainos wallet history --tx-hash <hash> --chain-index 501` | Transaction confirmation |

---

## Troubleshooting

| Issue | Solution |
|---|---|
| "FATAL: onchainos CLI not found" | Install onchainos and ensure it is in PATH |
| "No Solana address" | Run `onchainos wallet login <email>` to complete login |
| Login expired | Re-run `onchainos wallet login <email>` |
| Dashboard won't open | Check if port 3248 is in use: `lsof -i:3248` |
| Bot starts but doesn't trade | Check `PAUSED = True`, change to `False` (hot-reload, no restart needed) |
| Lots of SKIP in Feed | Signal tokens didn't pass pre-filter (MC/Liq/soldRatio), this is normal |
| Lots of SAFETY_REJECT in Feed | Deep verification blocked, adjust config based on rejection reason (DevRug/Bundler/K1) |
| `No signals` keeps appearing | No smart money buy signals currently, normal behavior — just wait |
| Live mode buy failure | Check SOL balance >= position + 0.05; confirm wallet is logged in |
| `InstructionError Custom:1` | `swap --from` must use native SOL `11111111111111111111111111111111`, not WSOL |
| SESSION_PAUSE, not trading | Session risk control triggered, wait for pause to end or adjust `SESSION_LOSS_LIMIT_SOL` |
| SESSION_STOP | Cumulative loss reached limit, restart bot to reset session |
| PnL display abnormal | Check `entry_price` field in `positions.json` for value of 0 |
| Config change not taking effect | No restart needed — `importlib.reload(config)` auto hot-reloads each cycle |

### Common Pitfalls

| Issue | Wrong Approach | Correct Approach |
|---|---|---|
| TP not profitable | TP uses raw pct | `tp_threshold = pct×100 + breakeven_pct` |
| Ignoring costs | TP at 8% and sell | NET 5% actual trigger = 5%+12% = 17% raw gain (low tier) |
| dev rug | Don't check dev | `onchainos token advanced-info` zero tolerance |
| Continuous losses without stopping | Keep trading | 3 consecutive losses pause / 0.10 SOL stop |
| swap --from uses WSOL | `So11111...112` | **Must use native SOL `1111...1111` (32 ones)** |
| contract-call --to | Pass token address | **Must pass swap response's `tx.to` (DEX router address)** |
| swap amount unit | Pass UI units | `swap quote/swap` `--amount` uses **lamports** (1 SOL = 1e9) |
| wallet balance get SOL | Get WSOL balance | Get entry where `tokenAddress === ''` = native SOL |

---

## Glossary

| Term | Definition |
|------|------|
| **Co-rider address** | Multiple smart wallets buying the same token within a short time, forming a "co-riding" consensus |
| **walletType** | Signal source type: 1=SmartMoney 2=KOL/Influencer 3=Whale |
| **triggerWalletCount** | Number of co-rider wallets; more wallets = stronger signal |
| **soldRatioPercent** | Smart Money sell ratio; >80% means smart money is already exiting |
| **breakeven_pct** | Breakeven point (including fees); varies by tier (7%-12%) |
| **Cost-Aware TP** | Take profit threshold = NET target + breakeven_pct, ensuring actual profit after fees |
| **Trailing Stop** | Price reaches activation threshold, then triggers sell when drawdown from peak exceeds threshold |
| **Time-decay SL** | Time-decay stop loss — the longer the position is held, the tighter the stop loss |
| **Trend Stop** | Trend time stop — based on 15m candle trend reversal detection |
| **Session Risk** | Per-run cumulative risk control (consecutive loss pause, cumulative loss stop) |
| **Fail-Closed** | When safety check API fails, treat as unsafe and do not buy |
| **TEE** | Trusted Execution Environment — onchainos signing happens inside a secure enclave |
| **Agentic Wallet** | onchainos managed wallet; private key stays inside TEE, never leaves the secure environment |
| **Dust** | Fragment position — residual holding valued below $0.10, automatically cleaned up |
| **Hot-reload** | `importlib.reload(config)` — auto-loads latest config.py parameters each cycle, no restart needed |
| **MC / MCAP** | Market Cap — token total supply × current price, measures token scale |
| **LP** | Liquidity Pool — token pair pool on DEX for trading; larger LP means lower slippage |
| **LP Burn** | Permanently destroying LP tokens, ensuring liquidity cannot be withdrawn by the developer |
| **Rug Pull** | Developer suddenly withdraws liquidity or dumps all holdings, causing token price to drop to zero |
| **Dev** | Token developer/deployer — in the meme coin context, refers to the token contract creator; their holdings and history are key risk indicators |
| **Bundler** | Bundle trader — addresses that buy large amounts through bundled transactions at token launch, possibly insiders or manipulators |
| **Sniper** | Addresses that automatically buy tokens at the instant of launch via bots; concentrated holdings may create sell pressure |
| **Honeypot** | Malicious token contract that can only be bought but not sold (or has extremely high sell tax) |
| **Slippage** | Difference between expected and actual execution price; worse liquidity means higher slippage |
| **K1** | Most recent 1-minute candle — used to detect short-term price spikes (K1 pump), preventing buying at the top |
| **lamports** | Smallest unit of SOL, 1 SOL = 1,000,000,000 lamports |
| **Native SOL** | SOL native token address `11111111111111111111111111111111` (32 ones); must use this address for swap --from |
| **WSOL** | Wrapped SOL (So11...112), SPL Token wrapped form of SOL; cannot be used for swap --from |
