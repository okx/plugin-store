//! Ranking sniper engine — pure functions, no I/O.
//!
//! Ported from Python prototype: skills/strategy-ranking-sniper/engine.py

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Constants ───────────────────────────────────────────────────────

pub const CHAIN: &str = "solana";
pub const CHAIN_INDEX: &str = "501";
pub const SOL_NATIVE: &str = "11111111111111111111111111111111";
pub const SOL_DECIMALS: u32 = 9;
pub const GAS_RESERVE_SOL: f64 = 0.01;
pub const MAX_POSITIONS: usize = 5;
pub const DAILY_LOSS_LIMIT_PCT: f64 = 15.0;
pub const TOP_N: usize = 20;
pub const TICK_INTERVAL_SECS: u64 = 10;
pub const SLIPPAGE_PCT: &str = "3";
pub const COOLDOWN_MINUTES: u64 = 30;
pub const MIN_WALLET_BALANCE: f64 = 0.1;

// Slot guard thresholds
pub const MAX_CHANGE_PCT: f64 = 150.0;
pub const MIN_CHANGE_PCT: f64 = 15.0;
pub const MIN_LIQUIDITY: f64 = 5000.0;
pub const MIN_MARKET_CAP: f64 = 5000.0;
pub const MAX_MARKET_CAP: f64 = 10_000_000.0;
pub const MIN_HOLDERS: i64 = 30;
pub const MIN_BUY_RATIO: f64 = 0.55;
pub const MIN_TRADERS: i64 = 20;

// Advanced safety thresholds
pub const MAX_RISK_LEVEL: i64 = 1;
pub const MAX_TOP10_HOLD: f64 = 50.0;
pub const MAX_DEV_HOLD: f64 = 20.0;
pub const MAX_BUNDLER_HOLD: f64 = 15.0;
pub const MIN_LP_BURN: f64 = 80.0;
pub const MAX_DEV_RUG_COUNT: i64 = 10;
pub const MAX_SNIPER_HOLD: f64 = 20.0;
pub const BLOCK_INTERNAL: bool = true;

// Holder risk scan thresholds
pub const MAX_SUSPICIOUS_HOLD: f64 = 10.0;
pub const MAX_SUSPICIOUS_COUNT: usize = 5;
pub const BLOCK_PHISHING: bool = true;

// Exit thresholds
pub const HARD_STOP_PCT: f64 = -25.0;
pub const FAST_STOP_TIME_SECS: u64 = 300; // 5 minutes
pub const FAST_STOP_PCT: f64 = -8.0;
pub const TRAILING_ACTIVATE_PCT: f64 = 8.0;
pub const TRAILING_DRAWDOWN_PCT: f64 = 12.0;
pub const TIME_STOP_SECS: u64 = 21600; // 6 hours
pub const TP_LEVELS: [f64; 3] = [5.0, 15.0, 30.0]; // gradient take-profit

// Momentum Score
pub const SCORE_BUY_THRESHOLD: u32 = 40;

// Circuit breaker
pub const MAX_CONSECUTIVE_ERRORS: u32 = 5;
pub const COOLDOWN_AFTER_ERRORS: u64 = 3600;

// Default budget
pub const DEFAULT_BUDGET_SOL: f64 = 0.5;
pub const DEFAULT_PER_TRADE_SOL: f64 = 0.05;

// State limits
pub const MAX_TRADES: usize = 100;

// Skip tokens (system addresses)
pub const SKIP_TOKENS: &[&str] = &[
    "11111111111111111111111111111111",
    "So11111111111111111111111111111111111111112",
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
];

// ── Data Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub token_address: String,
    pub symbol: String,
    pub buy_price: f64,
    pub buy_amount_sol: f64,
    pub buy_time: String, // RFC3339
    pub peak_pnl_pct: f64,
    pub trailing_active: bool,
    pub tp_sold: Vec<usize>, // which TP levels have been hit
    pub tx_hash: String,
    #[serde(default)]
    pub amount_raw: String, // raw token amount received from buy (for sell)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub time: String,
    pub symbol: String,
    pub token_address: String,
    pub action: String, // "BUY" or "SELL"
    pub price: f64,
    pub amount_sol: f64,
    pub score: Option<u32>,
    pub exit_reason: Option<String>,
    pub pnl_pct: Option<f64>,
    pub pnl_sol: Option<f64>,
    pub tx_hash: String,
}

/// Exit decision from the 6-layer system.
#[derive(Debug, Clone)]
pub struct ExitSignal {
    pub reason: String,
    /// "FULL" for full exit, "PARTIAL_25"/"PARTIAL_35"/"PARTIAL_40" for gradient TP
    pub exit_type: String,
}

// ── Slot Guard (13 checks from ranking data) ────────────────────────

/// Run 13-point slot guard from ranking data only.
/// Returns (passed, list of failure reasons).
pub fn run_slot_guard(
    token: &Value,
    positions_count: usize,
    is_holding: bool,
    daily_loss_exceeded: bool,
    cooldown_active: bool,
    cfg: &super::config::SniperConfig,
) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();

    let change = safe_float(&token["change"], 0.0);
    if change < cfg.min_change_pct {
        reasons.push(format!("change {change:.1}% < {}%", cfg.min_change_pct));
    }

    if change > cfg.max_change_pct {
        reasons.push(format!("change {change:.1}% > {}%", cfg.max_change_pct));
    }

    let liq = safe_float(&token["liquidity"], 0.0);
    if liq < cfg.min_liquidity {
        reasons.push(format!("liquidity ${liq:.0} < ${:.0}", cfg.min_liquidity));
    }

    let mc = safe_float(&token["marketCap"], 0.0);
    if mc < cfg.min_market_cap {
        reasons.push(format!("market cap ${mc:.0} < ${:.0}", cfg.min_market_cap));
    }

    if mc > cfg.max_market_cap {
        reasons.push(format!("market cap ${mc:.0} > ${:.0}", cfg.max_market_cap));
    }

    let holders = safe_int(&token["holders"], 0);
    if holders < cfg.min_holders {
        reasons.push(format!("holders {holders} < {}", cfg.min_holders));
    }

    let txs = safe_int(&token["txs"], 0);
    let txs_buy = safe_int(&token["txsBuy"], 0);
    let buy_ratio = if txs > 0 {
        txs_buy as f64 / txs as f64
    } else {
        0.0
    };
    if buy_ratio < cfg.min_buy_ratio {
        reasons.push(format!(
            "buy ratio {:.1}% < {:.0}%",
            buy_ratio * 100.0,
            cfg.min_buy_ratio * 100.0
        ));
    }

    let traders = safe_int(&token["uniqueTraders"], 0);
    if traders < cfg.min_traders {
        reasons.push(format!("unique traders {traders} < {}", cfg.min_traders));
    }

    let address = token["tokenAddress"]
        .as_str()
        .or_else(|| token["address"].as_str())
        .unwrap_or("");
    if SKIP_TOKENS.contains(&address) {
        reasons.push(format!("skip token: {address}"));
    }

    if cooldown_active {
        reasons.push("cooldown active".to_string());
    }

    if positions_count >= cfg.max_positions {
        reasons.push(format!(
            "max positions reached ({positions_count} >= {})",
            cfg.max_positions
        ));
    }

    if is_holding {
        reasons.push("already holding this token".to_string());
    }

    if daily_loss_exceeded {
        reasons.push("daily loss limit exceeded".to_string());
    }

    let passed = reasons.is_empty();
    (passed, reasons)
}

// ── Advanced Safety (9 checks from advanced-info API) ───────────────

/// Run 9-point advanced safety from advanced-info API.
/// Returns (passed, list of failure reasons).
pub fn run_advanced_safety(
    adv_info: &Value,
    cfg: &super::config::SniperConfig,
) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();

    let risk_level = safe_int(&adv_info["riskControlLevel"], 0);
    if risk_level > cfg.max_risk_level {
        reasons.push(format!("risk level {risk_level} > {}", cfg.max_risk_level));
    }

    if let Some(tags) = adv_info["tokenTags"].as_array() {
        let has_honeypot = tags
            .iter()
            .filter_map(|t| t.as_str())
            .any(|t| t.to_lowercase().contains("honeypot"));
        if has_honeypot {
            reasons.push("honeypot tag detected".to_string());
        }
    }

    let top10 = safe_float(&adv_info["top10HoldPercent"], 100.0);
    if top10 > cfg.max_top10_hold {
        reasons.push(format!(
            "top10 concentration {top10:.1}% > {}%",
            cfg.max_top10_hold
        ));
    }

    let dev_hold = safe_float(&adv_info["devHoldingPercent"], 0.0);
    if dev_hold > cfg.max_dev_hold {
        reasons.push(format!(
            "dev holding {dev_hold:.1}% > {}%",
            cfg.max_dev_hold
        ));
    }

    let bundler = safe_float(&adv_info["bundleHoldingPercent"], 0.0);
    if bundler > cfg.max_bundler_hold {
        reasons.push(format!("bundler {bundler:.1}% > {}%", cfg.max_bundler_hold));
    }

    let is_internal = adv_info["isInternal"]
        .as_bool()
        .or_else(|| {
            adv_info["isInternal"]
                .as_str()
                .map(|s| s == "true" || s == "1")
        })
        .unwrap_or(false);
    if !is_internal {
        let lp_burn = safe_float(&adv_info["lpBurnedPercent"], 0.0);
        if lp_burn < cfg.min_lp_burn {
            reasons.push(format!("LP burn {lp_burn:.0}% < {}%", cfg.min_lp_burn));
        }
    }

    let rug_count = safe_int(&adv_info["devRugPullTokenCount"], 0);
    if rug_count > cfg.max_dev_rug_count {
        reasons.push(format!(
            "dev rug count {rug_count} > {}",
            cfg.max_dev_rug_count
        ));
    }

    let sniper = safe_float(&adv_info["sniperHoldingPercent"], 0.0);
    if sniper > cfg.max_sniper_hold {
        reasons.push(format!("sniper {sniper:.1}% > {}%", cfg.max_sniper_hold));
    }

    if cfg.block_internal && is_internal {
        reasons.push("non-graduated PumpFun token (isInternal=true)".to_string());
    }

    let passed = reasons.is_empty();
    (passed, reasons)
}

// ── Holder Risk Scan (3 checks from token-holder API) ───────────────

/// Run holder risk scan from token-holder API data.
/// `suspicious_data` and `phishing_data` are JSON arrays of holder records,
/// each with a `holdPercent` field (string like "0.5" meaning 0.5%).
/// Returns (passed, list of failure reasons).
pub fn run_holder_risk_scan(
    suspicious_data: &Value,
    phishing_data: &Value,
    cfg: &super::config::SniperConfig,
) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();

    let empty_vec = vec![];
    let suspicious_active: Vec<f64> = suspicious_data
        .as_array()
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|h| {
            let pct = safe_float(&h["holdPercent"], 0.0);
            if pct > 0.0 {
                Some(pct)
            } else {
                None
            }
        })
        .collect();

    let suspicious_total: f64 = suspicious_active.iter().map(|p| p * 100.0).sum();
    if suspicious_total > cfg.max_suspicious_hold {
        reasons.push(format!(
            "suspicious hold total {suspicious_total:.1}% > {}%",
            cfg.max_suspicious_hold
        ));
    }

    if cfg.block_phishing {
        let phishing_active_count = phishing_data
            .as_array()
            .unwrap_or(&empty_vec)
            .iter()
            .filter(|h| safe_float(&h["holdPercent"], 0.0) > 0.0)
            .count();
        if phishing_active_count > 0 {
            reasons.push(format!(
                "phishing holders detected ({phishing_active_count})"
            ));
        }
    }

    if suspicious_active.len() > cfg.max_suspicious_count {
        reasons.push(format!(
            "suspicious holder count {} > {}",
            suspicious_active.len(),
            cfg.max_suspicious_count
        ));
    }

    let passed = reasons.is_empty();
    (passed, reasons)
}

// ── Momentum Score ──────────────────────────────────────────────────

/// Calculate momentum score (0-125). Higher = better signal.
///
/// Base Score (0-100): buyScore + changePenalty + traderScore + liqScore
/// Bonus Score (0-25): smart money, concentration, dsPaid, community, sniper, dev, suspicious
pub fn calc_momentum_score(token: &Value, adv_info: &Value, suspicious_active_count: usize) -> u32 {
    // ── Base Score (0-100) ──
    let change = safe_float(&token["change"], 0.0);
    let txs = safe_int(&token["txs"], 0).max(1) as f64;
    let txs_buy = safe_int(&token["txsBuy"], 0) as f64;
    let buy_ratio = (txs_buy / txs).min(1.0);
    let traders = safe_int(&token["uniqueTraders"], 0) as f64;
    let liquidity = safe_float(&token["liquidity"], 0.0);

    let buy_score = buy_ratio * 40.0;

    let change_penalty = if change > 100.0 {
        (20.0 - (change - 100.0) / 10.0).max(0.0)
    } else {
        (change / 5.0).min(20.0)
    };

    let trader_score = (traders / 50.0).min(1.0) * 20.0;
    let liq_score = (liquidity / 50000.0).min(1.0) * 20.0;

    let base = buy_score + change_penalty + trader_score + liq_score;

    // ── Bonus Score (0-25) ──
    let tags: Vec<&str> = adv_info["tokenTags"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|t| t.as_str()).collect())
        .unwrap_or_default();

    let smart_money_bonus: u32 = if tags.contains(&"smartMoneyBuy") {
        8
    } else {
        0
    };

    let top10 = safe_float(&adv_info["top10HoldPercent"], 100.0);
    let concentration_bonus: u32 = if top10 < 30.0 {
        5
    } else if top10 < 50.0 {
        2
    } else {
        0
    };

    let ds_paid_bonus: u32 = if tags.contains(&"dsPaid") { 3 } else { 0 };

    let community_bonus: u32 = if tags.contains(&"dexScreenerTokenCommunityTakeOver") {
        2
    } else {
        0
    };

    let sniper = safe_float(&adv_info["sniperHoldingPercent"], 0.0);
    let low_sniper_bonus: u32 = if sniper < 5.0 {
        4
    } else if sniper < 10.0 {
        2
    } else {
        0
    };

    let dev_hold = safe_float(&adv_info["devHoldingPercent"], 0.0);
    let dev_rug = safe_int(&adv_info["devRugPullTokenCount"], 0);
    let dev_clean_bonus: u32 = if dev_hold == 0.0 && dev_rug < 3 { 3 } else { 0 };

    let zero_suspicious_bonus: u32 = if suspicious_active_count == 0 { 2 } else { 0 };

    let bonus_total = (smart_money_bonus
        + concentration_bonus
        + ds_paid_bonus
        + community_bonus
        + low_sniper_bonus
        + dev_clean_bonus
        + zero_suspicious_bonus)
        .min(25);

    (base as u32) + bonus_total
}

// ── 6-Layer Exit System ─────────────────────────────────────────────

/// Check 6-layer exit system. Returns exit signal or None.
/// Priority: ranking > hard stop > fast stop > trailing > time stop > TP
///
/// `pos` is mutated to update peak_pnl_pct and trailing_active state.
pub fn check_exits(
    pos: &mut Position,
    current_price: f64,
    current_ranking: &std::collections::HashSet<String>,
    now_ts: i64,
    cfg: &super::config::SniperConfig,
) -> Option<ExitSignal> {
    if pos.buy_price <= 0.0 {
        return None;
    }

    let pnl_pct = (current_price - pos.buy_price) / pos.buy_price * 100.0;
    let buy_ts = chrono::DateTime::parse_from_rfc3339(&pos.buy_time)
        .map(|t| t.timestamp())
        .unwrap_or(0);
    let elapsed = (now_ts - buy_ts).max(0) as u64;

    // Update peak PnL
    if pnl_pct > pos.peak_pnl_pct {
        pos.peak_pnl_pct = pnl_pct;
    }

    // Layer 1: Ranking exit — token dropped off the ranking
    if !current_ranking.contains(&pos.token_address) && elapsed > 60 {
        return Some(ExitSignal {
            reason: format!("RANKING_EXIT (no longer in top {})", cfg.top_n),
            exit_type: "FULL".to_string(),
        });
    }

    // Layer 2: Hard stop-loss
    if pnl_pct <= cfg.hard_stop_pct {
        return Some(ExitSignal {
            reason: format!("HARD_STOP ({pnl_pct:+.1}% <= {}%)", cfg.hard_stop_pct),
            exit_type: "FULL".to_string(),
        });
    }

    // Layer 3: Fast stop
    if elapsed >= cfg.fast_stop_time_secs && pnl_pct <= cfg.fast_stop_pct {
        return Some(ExitSignal {
            reason: format!("FAST_STOP ({pnl_pct:+.1}% after {elapsed}s)"),
            exit_type: "FULL".to_string(),
        });
    }

    // Layer 4: Trailing stop
    if pnl_pct >= cfg.trailing_activate_pct {
        pos.trailing_active = true;
    }
    if pos.trailing_active {
        let drawdown = pos.peak_pnl_pct - pnl_pct;
        if drawdown >= cfg.trailing_drawdown_pct {
            return Some(ExitSignal {
                reason: format!(
                    "TRAILING_STOP (peak {:.1}%, now {pnl_pct:+.1}%, dd {drawdown:.1}%)",
                    pos.peak_pnl_pct
                ),
                exit_type: "FULL".to_string(),
            });
        }
    }

    // Layer 5: Time stop
    if elapsed >= cfg.time_stop_secs {
        return Some(ExitSignal {
            reason: format!("TIME_STOP ({:.1}h)", elapsed as f64 / 3600.0),
            exit_type: "FULL".to_string(),
        });
    }

    // Layer 6: Gradient take-profit (25%/35%/40%)
    let tp_exit_types = ["PARTIAL_25", "PARTIAL_35", "PARTIAL_40"];
    for (i, &tp_pct) in cfg.tp_levels.iter().enumerate() {
        if !pos.tp_sold.contains(&i) && pnl_pct >= tp_pct {
            pos.tp_sold.push(i);
            return Some(ExitSignal {
                reason: format!("TAKE_PROFIT_L{} (+{pnl_pct:.1}% >= +{tp_pct}%)", i + 1),
                exit_type: tp_exit_types[i].to_string(),
            });
        }
    }

    None
}

/// Check daily loss limit. Returns Some(reason) if breached.
pub fn check_daily_loss(
    daily_pnl_sol: f64,
    budget_sol: f64,
    daily_loss_limit_pct: f64,
) -> Option<String> {
    if budget_sol <= 0.0 {
        return None;
    }
    let loss_pct = daily_pnl_sol / budget_sol * 100.0;
    if loss_pct < -daily_loss_limit_pct {
        Some(format!(
            "Daily loss limit: {daily_pnl_sol:.4} SOL ({loss_pct:.1}% > -{daily_loss_limit_pct}%)"
        ))
    } else {
        None
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Safely parse a JSON value to f64, handling empty strings and nulls.
pub fn safe_float(val: &Value, default: f64) -> f64 {
    match val {
        Value::Number(n) => n.as_f64().unwrap_or(default),
        Value::String(s) if s.is_empty() => default,
        Value::String(s) => s.parse().unwrap_or(default),
        _ => default,
    }
}

/// Safely parse a JSON value to i64.
pub fn safe_int(val: &Value, default: i64) -> i64 {
    match val {
        Value::Number(n) => n.as_i64().unwrap_or(default),
        Value::String(s) if s.is_empty() => default,
        Value::String(s) => s.parse::<f64>().unwrap_or(default as f64) as i64,
        _ => default,
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::config::SniperConfig;
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    fn default_cfg() -> SniperConfig {
        // Use production-like values for tests (engine constants are relaxed for testing)
        let mut cfg = SniperConfig::default();
        cfg.min_change_pct = 15.0;
        cfg.max_change_pct = 150.0;
        cfg.min_liquidity = 5000.0;
        cfg.min_market_cap = 5000.0;
        cfg.max_market_cap = 10_000_000.0;
        cfg.min_holders = 30;
        cfg.min_buy_ratio = 0.55;
        cfg.min_traders = 20;
        cfg.max_risk_level = 1;
        cfg.max_top10_hold = 50.0;
        cfg.max_dev_hold = 20.0;
        cfg.max_bundler_hold = 15.0;
        cfg.min_lp_burn = 80.0;
        cfg.max_dev_rug_count = 10;
        cfg.max_sniper_hold = 20.0;
        cfg.block_internal = true;
        cfg.max_suspicious_hold = 10.0;
        cfg.max_suspicious_count = 5;
        cfg.block_phishing = true;
        cfg.fast_stop_time_secs = 300;
        cfg.time_stop_secs = 21600; // 6h production value
        cfg
    }

    #[test]
    fn test_safe_float_empty_string() {
        assert_eq!(safe_float(&json!(""), 0.0), 0.0);
        assert_eq!(safe_float(&json!("3.14"), 0.0), 3.14);
        assert_eq!(safe_float(&json!(42.5), 0.0), 42.5);
        assert_eq!(safe_float(&Value::Null, 99.0), 99.0);
    }

    #[test]
    fn test_safe_int_empty_string() {
        assert_eq!(safe_int(&json!(""), 0), 0);
        assert_eq!(safe_int(&json!("100"), 0), 100);
        assert_eq!(safe_int(&json!(42), 0), 42);
    }

    // ── Slot Guard ──────────────────────────────────────────────────

    #[test]
    fn test_slot_guard_passes_good_token() {
        let token = json!({
            "change": "30",
            "liquidity": "10000",
            "marketCap": "50000",
            "holders": "100",
            "txs": "200",
            "txsBuy": "130",
            "txsSell": "70",
            "uniqueTraders": "50",
            "tokenAddress": "SomeToken123",
        });
        let (passed, reasons) = run_slot_guard(&token, 2, false, false, false, &default_cfg());
        assert!(passed, "expected pass, got: {:?}", reasons);
    }

    #[test]
    fn test_slot_guard_fails_low_change() {
        let token = json!({
            "change": "5",
            "liquidity": "10000",
            "marketCap": "50000",
            "holders": "100",
            "txs": "200",
            "txsBuy": "130",
            "txsSell": "70",
            "uniqueTraders": "50",
        });
        let (passed, reasons) = run_slot_guard(&token, 0, false, false, false, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("change")));
    }

    #[test]
    fn test_slot_guard_fails_high_change() {
        let token = json!({
            "change": "200",
            "liquidity": "10000",
            "marketCap": "50000",
            "holders": "100",
            "txs": "200",
            "txsBuy": "130",
            "txsSell": "70",
            "uniqueTraders": "50",
        });
        let (passed, reasons) = run_slot_guard(&token, 0, false, false, false, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("150")));
    }

    #[test]
    fn test_slot_guard_fails_skip_token() {
        let token = json!({
            "change": "30",
            "liquidity": "10000",
            "marketCap": "50000",
            "holders": "100",
            "txs": "200",
            "txsBuy": "130",
            "txsSell": "70",
            "uniqueTraders": "50",
            "tokenAddress": "So11111111111111111111111111111111111111112",
        });
        let (passed, reasons) = run_slot_guard(&token, 0, false, false, false, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("skip token")));
    }

    #[test]
    fn test_slot_guard_fails_cooldown() {
        let token = json!({
            "change": "30",
            "liquidity": "10000",
            "marketCap": "50000",
            "holders": "100",
            "txs": "200",
            "txsBuy": "130",
            "txsSell": "70",
            "uniqueTraders": "50",
        });
        let (passed, reasons) = run_slot_guard(&token, 0, false, false, true, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("cooldown")));
    }

    #[test]
    fn test_slot_guard_fails_max_positions() {
        let token = json!({
            "change": "30",
            "liquidity": "10000",
            "marketCap": "50000",
            "holders": "100",
            "txs": "200",
            "txsBuy": "130",
            "txsSell": "70",
            "uniqueTraders": "50",
        });
        let (passed, reasons) = run_slot_guard(&token, 5, false, false, false, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("max positions")));
    }

    #[test]
    fn test_slot_guard_fails_already_holding() {
        let token = json!({
            "change": "30",
            "liquidity": "10000",
            "marketCap": "50000",
            "holders": "100",
            "txs": "200",
            "txsBuy": "130",
            "txsSell": "70",
            "uniqueTraders": "50",
        });
        let (passed, reasons) = run_slot_guard(&token, 0, true, false, false, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("already holding")));
    }

    #[test]
    fn test_slot_guard_fails_daily_loss() {
        let token = json!({
            "change": "30",
            "liquidity": "10000",
            "marketCap": "50000",
            "holders": "100",
            "txs": "200",
            "txsBuy": "130",
            "txsSell": "70",
            "uniqueTraders": "50",
        });
        let (passed, reasons) = run_slot_guard(&token, 0, false, true, false, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("daily loss")));
    }

    // ── Advanced Safety ─────────────────────────────────────────────

    #[test]
    fn test_advanced_safety_passes_good_token() {
        let adv = json!({
            "riskControlLevel": "1",
            "tokenTags": ["dsPaid"],
            "top10HoldPercent": "30",
            "devHoldingPercent": "5",
            "bundleHoldingPercent": "10",
            "lpBurnedPercent": "90",
            "devRugPullTokenCount": "2",
            "sniperHoldingPercent": "8",
            "isInternal": false,
        });
        let (passed, reasons) = run_advanced_safety(&adv, &default_cfg());
        assert!(passed, "expected pass, got: {:?}", reasons);
    }

    #[test]
    fn test_advanced_safety_fails_honeypot() {
        let adv = json!({
            "riskControlLevel": "1",
            "tokenTags": ["honeypot"],
            "top10HoldPercent": "30",
            "devHoldingPercent": "5",
            "bundleHoldingPercent": "10",
            "lpBurnedPercent": "90",
            "devRugPullTokenCount": "2",
            "sniperHoldingPercent": "8",
            "isInternal": false,
        });
        let (passed, reasons) = run_advanced_safety(&adv, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("honeypot")));
    }

    #[test]
    fn test_advanced_safety_fails_high_risk() {
        let adv = json!({
            "riskControlLevel": "2",
            "tokenTags": [],
            "top10HoldPercent": "30",
            "devHoldingPercent": "5",
            "bundleHoldingPercent": "10",
            "lpBurnedPercent": "90",
            "devRugPullTokenCount": "2",
            "sniperHoldingPercent": "8",
            "isInternal": false,
        });
        let (passed, reasons) = run_advanced_safety(&adv, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("risk level")));
    }

    #[test]
    fn test_advanced_safety_fails_internal_token() {
        let adv = json!({
            "riskControlLevel": "0",
            "tokenTags": [],
            "top10HoldPercent": "30",
            "devHoldingPercent": "5",
            "bundleHoldingPercent": "10",
            "lpBurnedPercent": "50",
            "devRugPullTokenCount": "2",
            "sniperHoldingPercent": "8",
            "isInternal": true,
        });
        let (passed, reasons) = run_advanced_safety(&adv, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("non-graduated")));
    }

    #[test]
    fn test_advanced_safety_lp_burn_skipped_for_internal() {
        // isInternal=true skips LP burn check (but fails on BLOCK_INTERNAL)
        let adv = json!({
            "riskControlLevel": "0",
            "tokenTags": [],
            "top10HoldPercent": "30",
            "devHoldingPercent": "5",
            "bundleHoldingPercent": "10",
            "lpBurnedPercent": "0",
            "devRugPullTokenCount": "2",
            "sniperHoldingPercent": "8",
            "isInternal": true,
        });
        let (_, reasons) = run_advanced_safety(&adv, &default_cfg());
        // Should NOT have LP burn failure (skipped for internal tokens)
        assert!(!reasons.iter().any(|r| r.contains("LP burn")));
        // But should have the isInternal block
        assert!(reasons.iter().any(|r| r.contains("non-graduated")));
    }

    // ── Holder Risk Scan ────────────────────────────────────────────

    #[test]
    fn test_holder_risk_scan_passes_clean() {
        let suspicious = json!([]);
        let phishing = json!([]);
        let (passed, reasons) = run_holder_risk_scan(&suspicious, &phishing, &default_cfg());
        assert!(passed, "expected pass, got: {:?}", reasons);
    }

    #[test]
    fn test_holder_risk_scan_fails_high_suspicious_hold() {
        let suspicious = json!([
            {"holdPercent": "0.05"},
            {"holdPercent": "0.08"},
        ]);
        let phishing = json!([]);
        let (passed, reasons) = run_holder_risk_scan(&suspicious, &phishing, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("suspicious hold total")));
    }

    #[test]
    fn test_holder_risk_scan_fails_phishing() {
        let suspicious = json!([]);
        let phishing = json!([
            {"holdPercent": "0.01"},
        ]);
        let (passed, reasons) = run_holder_risk_scan(&suspicious, &phishing, &default_cfg());
        assert!(!passed);
        assert!(reasons.iter().any(|r| r.contains("phishing")));
    }

    #[test]
    fn test_holder_risk_scan_fails_too_many_suspicious() {
        let suspicious = json!([
            {"holdPercent": "0.005"},
            {"holdPercent": "0.005"},
            {"holdPercent": "0.005"},
            {"holdPercent": "0.005"},
            {"holdPercent": "0.005"},
            {"holdPercent": "0.005"},
        ]);
        let phishing = json!([]);
        let (passed, reasons) = run_holder_risk_scan(&suspicious, &phishing, &default_cfg());
        assert!(!passed);
        assert!(reasons
            .iter()
            .any(|r| r.contains("suspicious holder count")));
    }

    #[test]
    fn test_holder_risk_scan_ignores_zero_hold() {
        let suspicious = json!([
            {"holdPercent": "0"},
            {"holdPercent": "0.0"},
        ]);
        let phishing = json!([
            {"holdPercent": "0"},
        ]);
        let (passed, reasons) = run_holder_risk_scan(&suspicious, &phishing, &default_cfg());
        assert!(
            passed,
            "expected pass (zero hold ignored), got: {:?}",
            reasons
        );
    }

    // ── Momentum Score ──────────────────────────────────────────────

    #[test]
    fn test_momentum_score_high() {
        let token = json!({
            "change": "50",
            "txs": "200",
            "txsBuy": "160",
            "uniqueTraders": "100",
            "liquidity": "60000",
        });
        let adv = json!({
            "tokenTags": ["smartMoneyBuy", "dsPaid"],
            "top10HoldPercent": "20",
            "sniperHoldingPercent": "3",
            "devHoldingPercent": "0",
            "devRugPullTokenCount": "1",
        });
        let score = calc_momentum_score(&token, &adv, 0);
        // buyScore: 0.8*40=32, changePenalty: 50/5=10, traderScore: min(100/50,1)*20=20,
        // liqScore: min(60000/50000,1)*20=20 => base=82
        // bonuses: smartMoney=8, concentration(20<30)=5, dsPaid=3, lowSniper(3<5)=4,
        //          devClean(0,1<3)=3, zeroSuspicious=2 => 25 (capped)
        // total = 82 + 25 = 107
        assert!(score >= 90, "expected high score, got {score}");
    }

    #[test]
    fn test_momentum_score_low() {
        let token = json!({
            "change": "16",
            "txs": "50",
            "txsBuy": "30",
            "uniqueTraders": "10",
            "liquidity": "5000",
        });
        let adv = json!({
            "tokenTags": [],
            "top10HoldPercent": "60",
            "sniperHoldingPercent": "15",
            "devHoldingPercent": "10",
            "devRugPullTokenCount": "5",
        });
        let score = calc_momentum_score(&token, &adv, 3);
        assert!(score < 50, "expected low score, got {score}");
    }

    #[test]
    fn test_momentum_score_change_penalty_high_change() {
        // change > 300 should yield 0 changePenalty (20 - (300-100)/10 = 0)
        let token = json!({
            "change": "300",
            "txs": "100",
            "txsBuy": "80",
            "uniqueTraders": "50",
            "liquidity": "50000",
        });
        let adv = json!({
            "tokenTags": [],
            "top10HoldPercent": "40",
            "sniperHoldingPercent": "8",
            "devHoldingPercent": "5",
            "devRugPullTokenCount": "5",
        });
        let score_high_change = calc_momentum_score(&token, &adv, 0);

        let token_normal = json!({
            "change": "50",
            "txs": "100",
            "txsBuy": "80",
            "uniqueTraders": "50",
            "liquidity": "50000",
        });
        let score_normal = calc_momentum_score(&token_normal, &adv, 0);
        // Normal change should score higher due to better changePenalty
        assert!(
            score_normal > score_high_change,
            "normal change {score_normal} should beat high change {score_high_change}"
        );
    }

    // ── Exit System ─────────────────────────────────────────────────

    #[test]
    fn test_exit_hard_stop() {
        let mut pos = Position {
            token_address: "abc".to_string(),
            symbol: "TEST".to_string(),
            buy_price: 1.0,
            buy_amount_sol: 0.05,
            buy_time: "2026-01-01T00:00:00Z".to_string(),
            peak_pnl_pct: 0.0,
            trailing_active: false,
            tp_sold: vec![],
            tx_hash: "tx1".to_string(),
            amount_raw: String::new(),
        };
        let ranking = HashSet::from(["abc".to_string()]);
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T01:00:00Z")
            .unwrap()
            .timestamp();
        let signal = check_exits(&mut pos, 0.7, &ranking, now, &default_cfg());
        assert!(signal.is_some());
        assert!(signal.unwrap().reason.contains("HARD_STOP"));
    }

    #[test]
    fn test_exit_ranking_drop() {
        let mut pos = Position {
            token_address: "abc".to_string(),
            symbol: "TEST".to_string(),
            buy_price: 1.0,
            buy_amount_sol: 0.05,
            buy_time: "2026-01-01T00:00:00Z".to_string(),
            peak_pnl_pct: 0.0,
            trailing_active: false,
            tp_sold: vec![],
            tx_hash: "tx1".to_string(),
            amount_raw: String::new(),
        };
        let ranking = HashSet::new(); // token not in ranking
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:05:00Z")
            .unwrap()
            .timestamp();
        let signal = check_exits(&mut pos, 1.0, &ranking, now, &default_cfg());
        assert!(signal.is_some());
        assert!(signal.unwrap().reason.contains("RANKING_EXIT"));
    }

    #[test]
    fn test_exit_fast_stop() {
        let mut pos = Position {
            token_address: "abc".to_string(),
            symbol: "TEST".to_string(),
            buy_price: 1.0,
            buy_amount_sol: 0.05,
            buy_time: "2026-01-01T00:00:00Z".to_string(),
            peak_pnl_pct: 0.0,
            trailing_active: false,
            tp_sold: vec![],
            tx_hash: "tx1".to_string(),
            amount_raw: String::new(),
        };
        let ranking = HashSet::from(["abc".to_string()]);
        // 6 minutes after buy (>= FAST_STOP_TIME_SECS=300s)
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:06:00Z")
            .unwrap()
            .timestamp();
        // Price at -10% (below FAST_STOP_PCT=-8%)
        let signal = check_exits(&mut pos, 0.90, &ranking, now, &default_cfg());
        assert!(signal.is_some());
        assert!(signal.unwrap().reason.contains("FAST_STOP"));
    }

    #[test]
    fn test_exit_fast_stop_not_triggered_before_time() {
        let mut pos = Position {
            token_address: "abc".to_string(),
            symbol: "TEST".to_string(),
            buy_price: 1.0,
            buy_amount_sol: 0.05,
            buy_time: "2026-01-01T00:00:00Z".to_string(),
            peak_pnl_pct: 0.0,
            trailing_active: false,
            tp_sold: vec![],
            tx_hash: "tx1".to_string(),
            amount_raw: String::new(),
        };
        let ranking = HashSet::from(["abc".to_string()]);
        // Only 3 minutes in (< 300s), fast stop should NOT trigger
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:03:00Z")
            .unwrap()
            .timestamp();
        let signal = check_exits(&mut pos, 0.90, &ranking, now, &default_cfg());
        assert!(
            signal.is_none(),
            "fast stop should not trigger before 5 min"
        );
    }

    #[test]
    fn test_exit_trailing_stop() {
        let mut pos = Position {
            token_address: "abc".to_string(),
            symbol: "TEST".to_string(),
            buy_price: 1.0,
            buy_amount_sol: 0.05,
            buy_time: "2026-01-01T00:00:00Z".to_string(),
            peak_pnl_pct: 15.0,
            trailing_active: true,
            tp_sold: vec![0], // TP L1 already hit
            tx_hash: "tx1".to_string(),
            amount_raw: String::new(),
        };
        let ranking = HashSet::from(["abc".to_string()]);
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T01:00:00Z")
            .unwrap()
            .timestamp();
        // Price at +2% (peak was +15%, drawdown = 13% > 12%)
        let signal = check_exits(&mut pos, 1.02, &ranking, now, &default_cfg());
        assert!(signal.is_some());
        assert!(signal.unwrap().reason.contains("TRAILING_STOP"));
    }

    #[test]
    fn test_exit_take_profit_partial_25() {
        let mut pos = Position {
            token_address: "abc".to_string(),
            symbol: "TEST".to_string(),
            buy_price: 1.0,
            buy_amount_sol: 0.05,
            buy_time: "2026-01-01T00:00:00Z".to_string(),
            peak_pnl_pct: 0.0,
            trailing_active: false,
            tp_sold: vec![],
            tx_hash: "tx1".to_string(),
            amount_raw: String::new(),
        };
        let ranking = HashSet::from(["abc".to_string()]);
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:30:00Z")
            .unwrap()
            .timestamp();
        let signal = check_exits(&mut pos, 1.06, &ranking, now, &default_cfg());
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert!(s.reason.contains("TAKE_PROFIT_L1"));
        assert_eq!(s.exit_type, "PARTIAL_25");
    }

    #[test]
    fn test_exit_take_profit_partial_35() {
        let mut pos = Position {
            token_address: "abc".to_string(),
            symbol: "TEST".to_string(),
            buy_price: 1.0,
            buy_amount_sol: 0.05,
            buy_time: "2026-01-01T00:00:00Z".to_string(),
            peak_pnl_pct: 0.0,
            trailing_active: false,
            tp_sold: vec![0], // L1 already sold
            tx_hash: "tx1".to_string(),
            amount_raw: String::new(),
        };
        let ranking = HashSet::from(["abc".to_string()]);
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:30:00Z")
            .unwrap()
            .timestamp();
        let signal = check_exits(&mut pos, 1.16, &ranking, now, &default_cfg());
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert!(s.reason.contains("TAKE_PROFIT_L2"));
        assert_eq!(s.exit_type, "PARTIAL_35");
    }

    #[test]
    fn test_exit_take_profit_partial_40() {
        let mut pos = Position {
            token_address: "abc".to_string(),
            symbol: "TEST".to_string(),
            buy_price: 1.0,
            buy_amount_sol: 0.05,
            buy_time: "2026-01-01T00:00:00Z".to_string(),
            peak_pnl_pct: 0.0,
            trailing_active: false,
            tp_sold: vec![0, 1], // L1 and L2 already sold
            tx_hash: "tx1".to_string(),
            amount_raw: String::new(),
        };
        let ranking = HashSet::from(["abc".to_string()]);
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-01T00:30:00Z")
            .unwrap()
            .timestamp();
        let signal = check_exits(&mut pos, 1.31, &ranking, now, &default_cfg());
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert!(s.reason.contains("TAKE_PROFIT_L3"));
        assert_eq!(s.exit_type, "PARTIAL_40");
    }

    // ── Daily Loss ──────────────────────────────────────────────────

    #[test]
    fn test_daily_loss_limit() {
        let cfg = default_cfg();
        assert!(check_daily_loss(-0.08, 0.5, cfg.daily_loss_limit_pct).is_some());
        assert!(check_daily_loss(-0.01, 0.5, cfg.daily_loss_limit_pct).is_none());
    }

    #[test]
    fn test_daily_loss_zero_budget() {
        let cfg = default_cfg();
        assert!(check_daily_loss(-1.0, 0.0, cfg.daily_loss_limit_pct).is_none());
    }
}
