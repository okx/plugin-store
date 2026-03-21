//! Memepump scanner engine — pure functions for filtering, signal detection, and exit logic.
//! No I/O, no state mutation. All thresholds from v2.4.1 strategy spec.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Chain (Solana, matching ranking_sniper) ─────────────────────────

pub const CHAIN_INDEX: &str = "501";
pub const SOL_NATIVE: &str = "11111111111111111111111111111111";
pub const SOL_DECIMALS: u32 = 9;

// ── Server-side filter params (14 total — API-enforced) ────────────

pub const TF_MIN_MC: u64 = 80_000;
pub const TF_MAX_MC: u64 = 800_000;
pub const TF_MIN_HOLDERS: u32 = 50;
pub const TF_MAX_DEV_HOLD: u32 = 10;
pub const TF_MAX_BUNDLER: u32 = 15;
pub const TF_MAX_SNIPER: u32 = 20;
pub const TF_MAX_INSIDER: u32 = 15;
pub const TF_MAX_TOP10: u32 = 50;
pub const TF_MAX_FRESH: u32 = 40;
pub const TF_MIN_TX: u32 = 30;
pub const TF_MIN_BUY_TX: u32 = 15;
pub const TF_MIN_AGE: u32 = 4;
pub const TF_MAX_AGE: u32 = 180;
pub const TF_MIN_VOL: u64 = 5_000;

// ── Client-side filter thresholds ───────────────────────────────────

pub const CF_MIN_BS_RATIO: f64 = 1.3;
pub const CF_MIN_VOL_MC_PCT: f64 = 5.0;
pub const CF_MAX_TOP10: f64 = 55.0;

// ── Deep safety thresholds ──────────────────────────────────────────

pub const DS_MAX_RUG: u32 = 0;
pub const DS_MAX_LAUNCHED: u32 = 20;
pub const DS_MAX_DEV_HOLD: f64 = 15.0;
pub const DS_MAX_BUNDLER_ATH: f64 = 25.0;
pub const DS_MAX_BUNDLER_COUNT: u32 = 5;

// ── Signal thresholds ───────────────────────────────────────────────

pub const SIG_A_RATIO_NORMAL: f64 = 1.35;
pub const SIG_A_RATIO_HOT: f64 = 1.2;
pub const SIG_A_FLOOR: u32 = 60;
pub const SIG_A_MIN_TX: u32 = 10;
pub const SIG_B_RATIO_QUIET: f64 = 1.5;
pub const SIG_B_RATIO_HOT: f64 = 2.0;
pub const SIG_C_MIN_BS: f64 = 1.5;
pub const HOT_VOL_THRESHOLD: f64 = 150_000_000.0;

// ── Position sizing ─────────────────────────────────────────────────

pub const SCALP_SOL: f64 = 0.0375;
pub const MINIMUM_SOL: f64 = 0.075;
pub const MAX_SOL: f64 = 0.15;
pub const MAX_POSITIONS: usize = 7;
pub const SOL_GAS_RESERVE: f64 = 0.05;
pub const SLIPPAGE_SCALP: u32 = 8;
pub const SLIPPAGE_MINIMUM: u32 = 10;

// ── Cost model ──────────────────────────────────────────────────────

pub const FIXED_COST_SOL: f64 = 0.001;
pub const COST_PER_LEG_PCT: f64 = 1.0;

// ── Exit params ─────────────────────────────────────────────────────

pub const TP1_PCT: f64 = 15.0;
pub const TP2_PCT: f64 = 25.0;
pub const TP1_SELL_SCALP: f64 = 0.60;
pub const TP1_SELL_HOT: f64 = 0.50;
pub const TP1_SELL_QUIET: f64 = 0.40;
pub const TP2_SELL_SCALP: f64 = 1.0;
pub const TP2_SELL_HOT: f64 = 1.0;
pub const TP2_SELL_QUIET: f64 = 0.80;
pub const SL_SCALP: f64 = -15.0;
pub const SL_HOT: f64 = -20.0;
pub const SL_QUIET: f64 = -25.0;
pub const EMERGENCY_SL: f64 = -50.0;
pub const TRAILING_PCT: f64 = 5.0;
pub const S3_SCALP_MIN: u64 = 5;
pub const S3_HOT_MIN: u64 = 8;
pub const S3_QUIET_MIN: u64 = 15;
pub const MAX_HOLD_MIN: u64 = 30;

// ── Session risk ────────────────────────────────────────────────────

pub const MAX_CONSEC_LOSS: u32 = 2;
pub const PAUSE_CONSEC_SEC: u64 = 900;
pub const PAUSE_LOSS_SOL: f64 = 0.05;
pub const STOP_LOSS_SOL: f64 = 0.10;
pub const MAX_CONSEC_ERRORS: u32 = 5;
pub const ERROR_COOLDOWN_SEC: u64 = 3600;

// ── Daemon ──────────────────────────────────────────────────────────

pub const TICK_INTERVAL_SECS: u64 = 10;
pub const API_DELAY_MS: u64 = 300;

// ── STUCK ───────────────────────────────────────────────────────────

pub const STUCK_MAX_FAILS: u32 = 5;

// ── State limits ────────────────────────────────────────────────────

pub const MAX_TRADES: usize = 200;
pub const MAX_SIGNALS: usize = 500;
pub const MAX_PREV_TX: usize = 500;

// ── Data Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub token_address: String,
    pub symbol: String,
    pub name: String,
    pub market_cap: f64,
    pub volume_1h: f64,
    pub buy_tx_1h: u32,
    pub sell_tx_1h: u32,
    pub holders: u32,
    pub top10_pct: f64,
    pub dev_hold_pct: f64,
    pub bundler_pct: f64,
    pub sniper_pct: f64,
    pub insider_pct: f64,
    pub fresh_wallet_pct: f64,
    pub created_timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SignalTier {
    Scalp,
    Minimum,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LaunchType {
    Hot,
    Quiet,
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub tier: SignalTier,
    pub launch: LaunchType,
    pub sig_a: bool,
    pub sig_a_ratio: f64,
    pub sig_b: bool,
    pub sig_b_ratio: f64,
    pub sig_c: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafetyVerdict {
    Safe,
    Unsafe(UnsafeReason),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnsafeReason {
    DevRug,
    DevFarm,
    DevHolding,
    BundlerAth,
    BundlerCount,
}

impl UnsafeReason {
    pub fn as_str(self) -> &'static str {
        match self {
            UnsafeReason::DevRug => "UNSAFE:DevRug",
            UnsafeReason::DevFarm => "UNSAFE:DevFarm",
            UnsafeReason::DevHolding => "UNSAFE:DevHolding",
            UnsafeReason::BundlerAth => "UNSAFE:BundlerAth",
            UnsafeReason::BundlerCount => "UNSAFE:BundlerCount",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExitAction {
    Emergency,
    StopLoss,
    TimeStop,
    TakeProfit1 { sell_pct: f64 },
    Breakeven,
    Trailing,
    TakeProfit2 { sell_pct: f64 },
    MaxHold,
}

impl ExitAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExitAction::Emergency => "EMERGENCY",
            ExitAction::StopLoss => "SL",
            ExitAction::TimeStop => "TIME_STOP",
            ExitAction::TakeProfit1 { .. } => "TP1",
            ExitAction::Breakeven => "BREAKEVEN",
            ExitAction::Trailing => "TRAILING",
            ExitAction::TakeProfit2 { .. } => "TP2",
            ExitAction::MaxHold => "MAX_HOLD",
        }
    }
}

pub struct ClassifyResult {
    pub bs_ratio: f64,
    pub vol_mc_pct: f64,
    pub top10: f64,
}

/// User-tunable exit thresholds, filled from ScannerConfig.
pub struct ExitParams {
    pub tp1_pct: f64,
    pub tp2_pct: f64,
    pub sl_scalp: f64,
    pub sl_hot: f64,
    pub sl_quiet: f64,
    pub trailing_pct: f64,
    pub max_hold_min: u64,
}

/// Parse a JSON value as f64, with a default fallback.
pub fn safe_float(val: &Value, default: f64) -> f64 {
    match val {
        Value::Number(n) => n.as_f64().unwrap_or(default),
        Value::String(s) => s.parse().unwrap_or(default),
        _ => default,
    }
}

/// Parse a JSON value as u32, with a default fallback.
pub fn safe_u32(val: &Value, default: u32) -> u32 {
    match val {
        Value::Number(n) => n.as_u64().unwrap_or(default as u64) as u32,
        Value::String(s) => s.parse().unwrap_or(default),
        _ => default,
    }
}

// ── Layer 2: Client-side pre-filter ─────────────────────────────────

pub fn classify_token(token: &TokenData) -> Option<ClassifyResult> {
    classify_token_with(token, CF_MIN_BS_RATIO, CF_MIN_VOL_MC_PCT, CF_MAX_TOP10)
}

pub fn classify_token_with(
    token: &TokenData,
    min_bs_ratio: f64,
    min_vol_mc_pct: f64,
    max_top10: f64,
) -> Option<ClassifyResult> {
    let bs_ratio = if token.sell_tx_1h > 0 {
        token.buy_tx_1h as f64 / token.sell_tx_1h as f64
    } else {
        f64::MAX
    };
    if bs_ratio < min_bs_ratio {
        return None;
    }

    let vol_mc_pct = if token.market_cap > 0.0 {
        token.volume_1h / token.market_cap * 100.0
    } else {
        0.0
    };
    if vol_mc_pct < min_vol_mc_pct {
        return None;
    }

    if token.top10_pct > max_top10 {
        return None;
    }

    Some(ClassifyResult {
        bs_ratio,
        vol_mc_pct,
        top10: token.top10_pct,
    })
}

// ── Layer 3: Deep safety check ──────────────────────────────────────

pub fn deep_safety_check(
    dev_rug_count: u32,
    dev_total_launched: u32,
    dev_holding_pct: f64,
    bundler_ath_pct: f64,
    bundler_count: u32,
) -> SafetyVerdict {
    deep_safety_check_with(
        dev_rug_count,
        dev_total_launched,
        dev_holding_pct,
        bundler_ath_pct,
        bundler_count,
        DS_MAX_DEV_HOLD,
        DS_MAX_BUNDLER_ATH,
        DS_MAX_BUNDLER_COUNT,
    )
}

/// Parameterized deep safety check with configurable thresholds.
#[allow(clippy::too_many_arguments)]
pub fn deep_safety_check_with(
    dev_rug_count: u32,
    dev_total_launched: u32,
    dev_holding_pct: f64,
    bundler_ath_pct: f64,
    bundler_count: u32,
    max_dev_hold: f64,
    max_bundler_ath: f64,
    max_bundler_count: u32,
) -> SafetyVerdict {
    if dev_rug_count > DS_MAX_RUG {
        return SafetyVerdict::Unsafe(UnsafeReason::DevRug);
    }
    if dev_total_launched > DS_MAX_LAUNCHED {
        return SafetyVerdict::Unsafe(UnsafeReason::DevFarm);
    }
    if dev_holding_pct > max_dev_hold {
        return SafetyVerdict::Unsafe(UnsafeReason::DevHolding);
    }
    if bundler_ath_pct > max_bundler_ath {
        return SafetyVerdict::Unsafe(UnsafeReason::BundlerAth);
    }
    if bundler_count > max_bundler_count {
        return SafetyVerdict::Unsafe(UnsafeReason::BundlerCount);
    }
    SafetyVerdict::Safe
}

// ── Launch type classification ──────────────────────────────────────

pub fn classify_launch(last_candle_volume: f64) -> LaunchType {
    if last_candle_volume > HOT_VOL_THRESHOLD {
        LaunchType::Hot
    } else {
        LaunchType::Quiet
    }
}

// ── Signal detection ────────────────────────────────────────────────

pub fn check_signal_a(
    current_min_tx: u32,
    elapsed_secs_in_min: u32,
    prev_min_tx: u32,
    launch: LaunchType,
) -> (bool, f64) {
    if prev_min_tx == 0 || elapsed_secs_in_min == 0 {
        return (false, 0.0);
    }

    let projection = (current_min_tx as f64 / elapsed_secs_in_min as f64) * 60.0;
    let ratio = projection / prev_min_tx as f64;

    let threshold = match launch {
        LaunchType::Hot => SIG_A_RATIO_HOT,
        LaunchType::Quiet => SIG_A_RATIO_NORMAL,
    };

    let triggered =
        (ratio >= threshold && current_min_tx >= SIG_A_MIN_TX) || projection >= SIG_A_FLOOR as f64;

    (triggered, ratio)
}

pub fn check_signal_b(
    current_1m_vol: f64,
    prev_5m_volumes: &[f64],
    launch: LaunchType,
) -> (bool, f64) {
    if prev_5m_volumes.is_empty() || current_1m_vol <= 0.0 {
        return (false, 0.0);
    }

    let avg = prev_5m_volumes.iter().sum::<f64>() / prev_5m_volumes.len() as f64;
    if avg <= 0.0 {
        return (false, 0.0);
    }

    let ratio = current_1m_vol / avg;
    let threshold = match launch {
        LaunchType::Hot => SIG_B_RATIO_HOT,
        LaunchType::Quiet => SIG_B_RATIO_QUIET,
    };

    (ratio >= threshold, ratio)
}

pub fn check_signal_c(buy_tx_1h: u32, sell_tx_1h: u32) -> bool {
    if sell_tx_1h == 0 {
        return buy_tx_1h > 0;
    }
    (buy_tx_1h as f64 / sell_tx_1h as f64) >= SIG_C_MIN_BS
}

pub fn detect_signal(sig_a: bool, sig_b: bool, sig_c: bool) -> Option<SignalTier> {
    if sig_a && sig_b && sig_c {
        Some(SignalTier::Minimum)
    } else if sig_a && sig_c {
        Some(SignalTier::Scalp)
    } else {
        None
    }
}

pub fn position_size(tier: SignalTier) -> f64 {
    match tier {
        SignalTier::Scalp => SCALP_SOL,
        SignalTier::Minimum => MINIMUM_SOL,
    }
}

pub fn slippage(tier: SignalTier) -> u32 {
    match tier {
        SignalTier::Scalp => SLIPPAGE_SCALP,
        SignalTier::Minimum => SLIPPAGE_MINIMUM,
    }
}

// ── Cost model ──────────────────────────────────────────────────────

pub fn calc_breakeven_pct(sol_amount: f64) -> f64 {
    if sol_amount <= 0.0 {
        return 100.0;
    }
    (FIXED_COST_SOL / sol_amount) * 100.0 + COST_PER_LEG_PCT * 2.0
}

// ── Exit system ─────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn check_exit(
    pnl_pct: f64,
    age_min: f64,
    peak_price: f64,
    current_price: f64,
    tp1_hit: bool,
    tier: SignalTier,
    launch: LaunchType,
    breakeven_pct: f64,
    ep: &ExitParams,
) -> Option<ExitAction> {
    if pnl_pct <= EMERGENCY_SL {
        return Some(ExitAction::Emergency);
    }
    if !tp1_hit {
        let sl = match (tier, launch) {
            (SignalTier::Scalp, _) => ep.sl_scalp,
            (_, LaunchType::Hot) => ep.sl_hot,
            (_, LaunchType::Quiet) => ep.sl_quiet,
        };
        if pnl_pct <= sl {
            return Some(ExitAction::StopLoss);
        }
    }
    if !tp1_hit && pnl_pct < 20.0 {
        let time_limit = match (tier, launch) {
            (SignalTier::Scalp, _) => S3_SCALP_MIN as f64,
            (_, LaunchType::Hot) => S3_HOT_MIN as f64,
            (_, LaunchType::Quiet) => S3_QUIET_MIN as f64,
        };
        if age_min >= time_limit {
            return Some(ExitAction::TimeStop);
        }
    }
    if !tp1_hit && pnl_pct >= ep.tp1_pct + breakeven_pct {
        let sell_pct = match (tier, launch) {
            (SignalTier::Scalp, _) => TP1_SELL_SCALP,
            (_, LaunchType::Hot) => TP1_SELL_HOT,
            (_, LaunchType::Quiet) => TP1_SELL_QUIET,
        };
        return Some(ExitAction::TakeProfit1 { sell_pct });
    }
    if tp1_hit && pnl_pct <= 0.0 {
        return Some(ExitAction::Breakeven);
    }
    if tp1_hit && peak_price > 0.0 && current_price > 0.0 {
        let drawdown_pct = (1.0 - current_price / peak_price) * 100.0;
        if drawdown_pct >= ep.trailing_pct {
            return Some(ExitAction::Trailing);
        }
    }
    if tp1_hit && pnl_pct >= ep.tp2_pct + breakeven_pct {
        let sell_pct = match (tier, launch) {
            (SignalTier::Scalp, _) => TP2_SELL_SCALP,
            (_, LaunchType::Hot) => TP2_SELL_HOT,
            (_, LaunchType::Quiet) => TP2_SELL_QUIET,
        };
        return Some(ExitAction::TakeProfit2 { sell_pct });
    }
    if age_min >= ep.max_hold_min as f64 {
        return Some(ExitAction::MaxHold);
    }
    None
}

pub fn exit_sell_pct(action: ExitAction) -> f64 {
    match action {
        ExitAction::Emergency => 1.0,
        ExitAction::StopLoss => 1.0,
        ExitAction::TimeStop => 1.0,
        ExitAction::TakeProfit1 { sell_pct } => sell_pct,
        ExitAction::Breakeven => 1.0,
        ExitAction::Trailing => 1.0,
        ExitAction::TakeProfit2 { sell_pct } => sell_pct,
        ExitAction::MaxHold => 1.0,
    }
}

// ── Session risk ────────────────────────────────────────────────────

pub fn check_session_risk(
    consecutive_losses: u32,
    session_loss_sol: f64,
    paused_until: Option<&str>,
    now: &str,
) -> Option<String> {
    if session_loss_sol >= STOP_LOSS_SOL {
        return Some(format!(
            "Session terminated: cumulative loss {:.4} SOL >= {:.2} SOL limit",
            session_loss_sol, STOP_LOSS_SOL
        ));
    }
    if let Some(until) = paused_until {
        if now < until {
            return Some(format!("Paused until {until}"));
        }
    }
    if consecutive_losses >= MAX_CONSEC_LOSS {
        return Some(format!(
            "{consecutive_losses} consecutive losses — pause {PAUSE_CONSEC_SEC}s"
        ));
    }
    if session_loss_sol >= PAUSE_LOSS_SOL {
        return Some(format!(
            "Session loss {:.4} SOL >= {:.2} SOL — pause 30min",
            session_loss_sol, PAUSE_LOSS_SOL
        ));
    }
    None
}

pub fn check_circuit_breaker(
    consecutive_errors: u32,
    last_error_time: Option<&str>,
    now: &str,
) -> Option<String> {
    if consecutive_errors >= MAX_CONSEC_ERRORS {
        if let Some(last_err) = last_error_time {
            if let (Ok(t), Ok(n)) = (
                chrono::DateTime::parse_from_rfc3339(last_err),
                chrono::DateTime::parse_from_rfc3339(now),
            ) {
                let elapsed = n.signed_duration_since(t).num_seconds() as u64;
                if elapsed < ERROR_COOLDOWN_SEC {
                    return Some(format!(
                        "Circuit breaker: {consecutive_errors} errors, cooldown {}s remaining",
                        ERROR_COOLDOWN_SEC - elapsed
                    ));
                }
            }
        }
    }
    None
}
