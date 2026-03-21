//! Signal tracker engine — pure functions, no I/O.
//!
//! Ported from Python strategy: docs/signal/SignalTracker_SKILL.md

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Constants ───────────────────────────────────────────────────────

pub const CHAIN_INDEX: &str = "501";
pub const SOL_NATIVE: &str = "11111111111111111111111111111111";
pub const SOL_DECIMALS: u32 = 9;
pub const TICK_INTERVAL_SECS: u64 = 20;

// Signal filter
pub const SIGNAL_LABELS: &str = "1,2,3"; // 1=SmartMoney 2=KOL 3=Whale
pub const MIN_WALLET_COUNT: u32 = 3;
pub const MAX_SELL_RATIO: f64 = 0.80;

// Safety thresholds
pub const MIN_MCAP: f64 = 200_000.0;
pub const MIN_LIQUIDITY: f64 = 80_000.0;
pub const MIN_HOLDERS: i64 = 300;
pub const MIN_LIQ_MC_RATIO: f64 = 0.05;
pub const MAX_TOP10_HOLDER_PCT: f64 = 50.0;
pub const MIN_LP_BURN: f64 = 80.0;
pub const MIN_HOLDER_DENSITY: f64 = 300.0; // per $1M MC
pub const MAX_K1_PUMP_PCT: f64 = 15.0;

// Dev/Bundler safety
pub const DEV_MAX_LAUNCHED: i64 = 20;
pub const DEV_MAX_HOLD_PCT: f64 = 15.0;
pub const BUNDLE_MAX_ATH_PCT: f64 = 25.0;
pub const BUNDLE_MAX_COUNT: i64 = 5;

// Position sizing
pub const POSITION_HIGH_SOL: f64 = 0.020;
pub const POSITION_MID_SOL: f64 = 0.015;
pub const POSITION_LOW_SOL: f64 = 0.010;
pub const WALLET_HIGH_THRESHOLD: u32 = 8;
pub const WALLET_MID_THRESHOLD: u32 = 5;
pub const MAX_POSITIONS: usize = 6;
pub const SLIPPAGE_PCT: &str = "1";
pub const GAS_RESERVE_SOL: f64 = 0.05;

// Cost model
pub const FIXED_COST_SOL: f64 = 0.001;
pub const COST_PER_LEG_PCT: f64 = 1.0;

// Take profit (net %, sell fraction)
pub const TP_TIERS: [(f64, f64); 3] = [
    (5.0, 0.30),  // TP1: +5% net, sell 30%
    (15.0, 0.40), // TP2: +15% net, sell 40%
    (30.0, 1.00), // TP3: +30% net, sell 100%
];

// Trailing stop
pub const TRAIL_ACTIVATE_PCT: f64 = 12.0;
pub const TRAIL_DISTANCE_PCT: f64 = 10.0;

// Stop loss
pub const SL_MULTIPLIER: f64 = 0.90; // -10%
pub const LIQ_EMERGENCY: f64 = 5_000.0;
pub const DUST_THRESHOLD_USD: f64 = 0.10;
pub const TIME_STOP_HOURS: f64 = 4.0;

// Time-decay SL (after_min, sl_pct as negative)
pub const TIME_DECAY_SL: [(u64, f64); 3] = [(60, -0.05), (30, -0.08), (15, -0.10)];

// Session risk
pub const MAX_CONSEC_LOSS: u32 = 3;
pub const PAUSE_CONSEC_SEC: u64 = 600;
pub const SESSION_LOSS_LIMIT_SOL: f64 = 0.05;
pub const SESSION_LOSS_PAUSE_SEC: u64 = 1800;
pub const SESSION_STOP_SOL: f64 = 0.10;

// Circuit breaker
pub const MAX_CONSECUTIVE_ERRORS: u32 = 5;
pub const COOLDOWN_AFTER_ERRORS: u64 = 3600;

// State limits
pub const MAX_TRADES: usize = 100;
pub const MAX_KNOWN_TOKENS: usize = 500;

// ── Data Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub token_address: String,
    pub symbol: String,
    pub label: String, // "SmartMoney", "KOL", "Whale"
    pub tier: String,  // "high", "mid", "low"
    pub buy_price: f64,
    pub buy_amount_sol: f64,
    pub buy_time: String,   // RFC3339
    pub breakeven_pct: f64, // breakeven % for this position
    pub peak_price: f64,
    pub peak_pnl_pct: f64,
    pub trailing_active: bool,
    pub tp_tier: usize, // next TP tier to check (0, 1, 2)
    pub entry_mc: f64,
    pub tx_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub time: String,
    pub symbol: String,
    pub token_address: String,
    pub label: String,
    pub tier: String,
    pub action: String, // "BUY" or "SELL"
    pub price: f64,
    pub amount_sol: f64,
    pub entry_mc: Option<f64>,
    pub exit_mc: Option<f64>,
    pub exit_reason: Option<String>,
    pub pnl_pct: Option<f64>,
    pub net_pnl_pct: Option<f64>,
    pub pnl_sol: Option<f64>,
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct ExitSignal {
    pub reason: String,
    pub sell_pct: f64, // 0.0-1.0, fraction to sell
}

// ── Position Tier ──────────────────────────────────────────────────

pub fn calc_position_tier(wallet_count: u32) -> (&'static str, f64) {
    if wallet_count >= WALLET_HIGH_THRESHOLD {
        ("high", POSITION_HIGH_SOL)
    } else if wallet_count >= WALLET_MID_THRESHOLD {
        ("mid", POSITION_MID_SOL)
    } else {
        ("low", POSITION_LOW_SOL)
    }
}

pub fn calc_breakeven(sol_amount: f64) -> f64 {
    if sol_amount <= 0.0 {
        return 0.0;
    }
    (FIXED_COST_SOL / sol_amount) * 100.0 + COST_PER_LEG_PCT * 2.0
}

// ── Helpers ─────────────────────────────────────────────────────────

pub fn safe_float(val: &Value, default: f64) -> f64 {
    match val {
        Value::Number(n) => n.as_f64().unwrap_or(default),
        Value::String(s) if s.is_empty() => default,
        Value::String(s) => s.parse().unwrap_or(default),
        _ => default,
    }
}

pub fn safe_int(val: &Value, default: i64) -> i64 {
    match val {
        Value::Number(n) => n.as_i64().unwrap_or(default),
        Value::String(s) if s.is_empty() => default,
        Value::String(s) => s.parse::<f64>().unwrap_or(default as f64) as i64,
        _ => default,
    }
}

pub fn wallet_type_label(wallet_type: &str) -> &'static str {
    match wallet_type {
        "1" | "SMART_MONEY" => "SmartMoney",
        "2" | "INFLUENCER" | "KOL" => "KOL",
        "3" | "WHALE" => "Whale",
        _ => "Unknown",
    }
}

// ── Signal Pre-filter ───────────────────────────────────────────────

pub fn run_signal_prefilter(signal: &Value) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();

    let wallet_count = safe_int(&signal["triggerWalletCount"], 0) as u32;
    if wallet_count < MIN_WALLET_COUNT {
        reasons.push(format!("wallet count {wallet_count} < {MIN_WALLET_COUNT}"));
    }

    let sold_ratio = safe_float(&signal["soldRatioPercent"], 100.0) / 100.0;
    if sold_ratio >= MAX_SELL_RATIO {
        reasons.push(format!(
            "sold ratio {:.0}% >= {:.0}%",
            sold_ratio * 100.0,
            MAX_SELL_RATIO * 100.0
        ));
    }

    let mc = safe_float(&signal["token"]["marketCapUsd"], 0.0);
    if mc > 0.0 && mc < MIN_MCAP {
        reasons.push(format!("MC ${mc:.0} < ${MIN_MCAP:.0}"));
    }

    let holders = safe_int(&signal["token"]["holders"], 0);
    if holders > 0 && holders < MIN_HOLDERS {
        reasons.push(format!("holders {holders} < {MIN_HOLDERS}"));
    }

    (reasons.is_empty(), reasons)
}

// ── Safety Checks ───────────────────────────────────────────────────

pub fn run_safety_checks(price_info: &Value) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();

    let mc = safe_float(&price_info["marketCap"], 0.0);
    let liq = safe_float(&price_info["liquidity"], 0.0);
    let holders = safe_int(&price_info["holders"], 0);

    if mc < MIN_MCAP {
        reasons.push(format!("MC ${mc:.0} < ${MIN_MCAP:.0}"));
    }
    if liq < MIN_LIQUIDITY {
        reasons.push(format!("Liq ${liq:.0} < ${MIN_LIQUIDITY:.0}"));
    }
    if holders < MIN_HOLDERS {
        reasons.push(format!("holders {holders} < {MIN_HOLDERS}"));
    }
    if mc > 0.0 {
        let liq_mc_ratio = liq / mc;
        if liq_mc_ratio < MIN_LIQ_MC_RATIO {
            reasons.push(format!(
                "liq/mc {:.1}% < {:.0}%",
                liq_mc_ratio * 100.0,
                MIN_LIQ_MC_RATIO * 100.0
            ));
        }
    }

    let top10 = safe_float(&price_info["top10HolderPercent"], 100.0);
    if top10 > MAX_TOP10_HOLDER_PCT {
        reasons.push(format!("top10 {top10:.1}% > {MAX_TOP10_HOLDER_PCT}%"));
    }
    if mc > 0.0 {
        let density = holders as f64 / (mc / 1_000_000.0);
        if density < MIN_HOLDER_DENSITY {
            reasons.push(format!("holder density {density:.0}/M < {MIN_HOLDER_DENSITY}/M"));
        }
    }

    let lp_burn = safe_float(&price_info["lpBurnedPercent"], 0.0);
    if lp_burn < MIN_LP_BURN {
        reasons.push(format!("LP burn {lp_burn:.0}% < {MIN_LP_BURN}%"));
    }

    (reasons.is_empty(), reasons)
}

// ── Dev/Bundler Checks ──────────────────────────────────────────────

pub fn run_dev_bundler_checks(dev_info: &Value, bundle_info: &Value) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();

    let rug_count = safe_int(&dev_info["rugPullCount"], 0);
    if rug_count > 0 {
        reasons.push(format!("dev rug count {rug_count} > 0"));
    }
    let launched = safe_int(&dev_info["tokenLaunchedCount"], 0);
    if launched > DEV_MAX_LAUNCHED {
        reasons.push(format!("dev launched {launched} > {DEV_MAX_LAUNCHED}"));
    }
    let dev_hold = safe_float(&dev_info["devHoldingPercent"], 0.0);
    if dev_hold > DEV_MAX_HOLD_PCT {
        reasons.push(format!("dev hold {dev_hold:.1}% > {DEV_MAX_HOLD_PCT}%"));
    }
    let bundler_ath = safe_float(&bundle_info["bundlerAthPercent"], 0.0);
    if bundler_ath > BUNDLE_MAX_ATH_PCT {
        reasons.push(format!("bundler ATH {bundler_ath:.1}% > {BUNDLE_MAX_ATH_PCT}%"));
    }
    let bundler_count = safe_int(&bundle_info["bundlerCount"], 0);
    if bundler_count > BUNDLE_MAX_COUNT {
        reasons.push(format!("bundler count {bundler_count} > {BUNDLE_MAX_COUNT}"));
    }

    (reasons.is_empty(), reasons)
}

pub fn check_k1_pump(candles: &Value) -> Option<String> {
    let arr = candles.as_array()?;
    if arr.is_empty() { return None; }
    let latest = arr.last()?;
    let items = latest.as_array()?;
    if items.len() < 5 { return None; }
    let open = safe_float(&items[1], 0.0);
    let close = safe_float(&items[4], 0.0);
    if open <= 0.0 { return None; }
    let change_pct = (close - open) / open * 100.0;
    if change_pct > MAX_K1_PUMP_PCT {
        Some(format!("1m pump {change_pct:.1}% > {MAX_K1_PUMP_PCT}%"))
    } else {
        None
    }
}

pub fn check_honeypot(quote: &Value) -> Option<String> {
    let is_honeypot = quote["isHoneyPot"]
        .as_bool()
        .or_else(|| quote["isHoneyPot"].as_str().map(|s| s == "true"))
        .unwrap_or(false);
    if is_honeypot {
        return Some("honeypot detected".to_string());
    }
    let tax_rate = safe_float(&quote["taxRate"], 0.0);
    if tax_rate > 5.0 {
        return Some(format!("tax rate {tax_rate:.1}% > 5%"));
    }
    None
}

// ── 7-Layer Exit System ─────────────────────────────────────────────

pub fn check_exits(
    pos: &mut Position,
    current_price: f64,
    current_liq: f64,
    _current_mc: f64,
    now_ts: i64,
) -> Option<ExitSignal> {
    if pos.buy_price <= 0.0 { return None; }

    let pnl_pct = (current_price - pos.buy_price) / pos.buy_price * 100.0;
    let buy_ts = chrono::DateTime::parse_from_rfc3339(&pos.buy_time)
        .map(|t| t.timestamp())
        .unwrap_or(0);
    let elapsed_secs = (now_ts - buy_ts).max(0) as u64;
    let elapsed_min = elapsed_secs / 60;

    if current_price > pos.peak_price { pos.peak_price = current_price; }
    if pnl_pct > pos.peak_pnl_pct { pos.peak_pnl_pct = pnl_pct; }

    // Layer 0: Liquidity emergency
    if current_liq < LIQ_EMERGENCY && current_liq > 0.0 {
        return Some(ExitSignal {
            reason: format!("RUG_LIQ (liq ${current_liq:.0} < ${LIQ_EMERGENCY:.0})"),
            sell_pct: 1.0,
        });
    }

    // Layer 1: Dust cleanup
    let current_value_sol = pos.buy_amount_sol * (1.0 + pnl_pct / 100.0);
    if current_value_sol < DUST_THRESHOLD_USD / 150.0 && current_value_sol > 0.0 {
        return Some(ExitSignal {
            reason: format!("DUST (est value {current_value_sol:.6} SOL)"),
            sell_pct: 1.0,
        });
    }

    // Layer 2: Time-decay SL
    if pos.tp_tier == 0 {
        for &(after_min, sl_pct) in TIME_DECAY_SL.iter() {
            if elapsed_min >= after_min && pnl_pct <= sl_pct * 100.0 {
                return Some(ExitSignal {
                    reason: format!(
                        "TIME_DECAY_SL ({pnl_pct:+.1}% <= {:.0}% after {elapsed_min}min)",
                        sl_pct * 100.0
                    ),
                    sell_pct: 1.0,
                });
            }
        }
    }

    // Layer 3: Hard stop-loss
    let sl_pct = (SL_MULTIPLIER - 1.0) * 100.0;
    if pnl_pct <= sl_pct {
        return Some(ExitSignal {
            reason: format!("HARD_SL ({pnl_pct:+.1}% <= {sl_pct:.0}%)"),
            sell_pct: 1.0,
        });
    }

    // Layer 4: Cost-aware take profit
    if pos.tp_tier < TP_TIERS.len() {
        let (net_target, sell_frac) = TP_TIERS[pos.tp_tier];
        let tp_threshold = net_target + pos.breakeven_pct;
        if pnl_pct >= tp_threshold {
            let net_pnl_pct = pnl_pct - pos.breakeven_pct;
            pos.tp_tier += 1;
            return Some(ExitSignal {
                reason: format!(
                    "TP{} (+{pnl_pct:.1}% >= +{tp_threshold:.1}%, net +{net_pnl_pct:.1}%)",
                    pos.tp_tier
                ),
                sell_pct: sell_frac,
            });
        }
    }

    // Layer 5: Trailing stop
    if pos.tp_tier >= 1 && pnl_pct >= TRAIL_ACTIVATE_PCT + pos.breakeven_pct {
        pos.trailing_active = true;
    }
    if pos.trailing_active {
        let drawdown = pos.peak_pnl_pct - pnl_pct;
        if drawdown >= TRAIL_DISTANCE_PCT {
            return Some(ExitSignal {
                reason: format!(
                    "TRAILING_STOP (peak {:.1}%, now {pnl_pct:+.1}%, dd {drawdown:.1}%)",
                    pos.peak_pnl_pct
                ),
                sell_pct: 1.0,
            });
        }
    }

    // Layer 6: Hard time stop
    let time_stop_secs = (TIME_STOP_HOURS * 3600.0) as u64;
    if elapsed_secs >= time_stop_secs {
        return Some(ExitSignal {
            reason: format!("TIME_STOP ({:.1}h)", elapsed_secs as f64 / 3600.0),
            sell_pct: 1.0,
        });
    }

    None
}

// ── Session Risk ────────────────────────────────────────────────────

pub fn check_session_risk(
    consecutive_losses: u32,
    cumulative_loss_sol: f64,
) -> Option<(String, u64)> {
    if cumulative_loss_sol >= SESSION_STOP_SOL {
        return Some((
            format!("SESSION_STOP: cumulative loss {cumulative_loss_sol:.4} SOL >= {SESSION_STOP_SOL} SOL"),
            u64::MAX,
        ));
    }
    if cumulative_loss_sol >= SESSION_LOSS_LIMIT_SOL {
        return Some((
            format!("SESSION_PAUSE: cumulative loss {cumulative_loss_sol:.4} SOL >= {SESSION_LOSS_LIMIT_SOL} SOL"),
            SESSION_LOSS_PAUSE_SEC,
        ));
    }
    if consecutive_losses >= MAX_CONSEC_LOSS {
        return Some((
            format!("CONSEC_PAUSE: {consecutive_losses} consecutive losses >= {MAX_CONSEC_LOSS}"),
            PAUSE_CONSEC_SEC,
        ));
    }
    None
}

// ── Config Summary ──────────────────────────────────────────────────

pub fn config_summary() -> serde_json::Value {
    serde_json::json!({
        "signal_filter": {
            "chain": "Solana (501)",
            "wallet_types": SIGNAL_LABELS,
            "min_wallet_count": MIN_WALLET_COUNT,
            "max_sell_ratio": format!("{:.0}%", MAX_SELL_RATIO * 100.0),
        },
        "safety_thresholds": {
            "min_mcap": format!("${:.0}", MIN_MCAP),
            "min_liquidity": format!("${:.0}", MIN_LIQUIDITY),
            "min_holders": MIN_HOLDERS,
            "min_liq_mc_ratio": format!("{:.0}%", MIN_LIQ_MC_RATIO * 100.0),
            "max_top10_holder_pct": format!("{:.0}%", MAX_TOP10_HOLDER_PCT),
            "min_lp_burn": format!("{:.0}%", MIN_LP_BURN),
        },
        "position_sizing": {
            "high_tier": format!("{} SOL (>={} wallets)", POSITION_HIGH_SOL, WALLET_HIGH_THRESHOLD),
            "mid_tier": format!("{} SOL (>={} wallets)", POSITION_MID_SOL, WALLET_MID_THRESHOLD),
            "low_tier": format!("{} SOL (>=3 wallets)", POSITION_LOW_SOL),
            "max_positions": MAX_POSITIONS,
        },
        "exit_system": {
            "hard_sl": format!("{:.0}%", (SL_MULTIPLIER - 1.0) * 100.0),
            "tp1": format!("+{}% net, sell {}%", TP_TIERS[0].0, TP_TIERS[0].1 * 100.0),
            "tp2": format!("+{}% net, sell {}%", TP_TIERS[1].0, TP_TIERS[1].1 * 100.0),
            "tp3": format!("+{}% net, sell {}%", TP_TIERS[2].0, TP_TIERS[2].1 * 100.0),
            "trailing_activate": format!("+{}%", TRAIL_ACTIVATE_PCT),
            "trailing_distance": format!("{}% drawdown", TRAIL_DISTANCE_PCT),
            "time_stop": format!("{}h", TIME_STOP_HOURS),
        },
        "session_risk": {
            "max_consecutive_losses": MAX_CONSEC_LOSS,
            "session_loss_pause_sol": SESSION_LOSS_LIMIT_SOL,
            "session_loss_stop_sol": SESSION_STOP_SOL,
        },
        "tick_interval": format!("{}s", TICK_INTERVAL_SECS),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_calc_position_tier() {
        assert_eq!(calc_position_tier(10), ("high", 0.020));
        assert_eq!(calc_position_tier(5), ("mid", 0.015));
        assert_eq!(calc_position_tier(3), ("low", 0.010));
    }

    #[test]
    fn test_signal_prefilter_passes() {
        let signal = json!({
            "triggerWalletCount": 5,
            "soldRatioPercent": "30",
            "token": { "marketCapUsd": "500000", "holders": "500" }
        });
        let (passed, reasons) = run_signal_prefilter(&signal);
        assert!(passed, "{:?}", reasons);
    }

    #[test]
    fn test_exit_hard_sl() {
        let mut pos = make_test_position(1.0, 0.010);
        let signal = check_exits(&mut pos, 0.89, 100_000.0, 500_000.0, buy_ts_plus(60));
        assert!(signal.is_some());
        assert!(signal.unwrap().reason.contains("HARD_SL"));
    }

    fn make_test_position(buy_price: f64, sol: f64) -> Position {
        Position {
            token_address: "TestToken".to_string(),
            symbol: "TEST".to_string(),
            label: "SmartMoney".to_string(),
            tier: "low".to_string(),
            buy_price,
            buy_amount_sol: sol,
            buy_time: "2026-01-01T00:00:00Z".to_string(),
            breakeven_pct: calc_breakeven(sol),
            peak_price: buy_price,
            peak_pnl_pct: 0.0,
            trailing_active: false,
            tp_tier: 0,
            entry_mc: 500_000.0,
            tx_hash: "tx_test".to_string(),
        }
    }

    fn buy_ts_plus(secs: i64) -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .timestamp()
            + secs
    }
}
