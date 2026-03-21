//! Signal tracker user-configurable parameters — persisted at ~/.plugin-store/signal_tracker_config.json.
//! Log file at ~/.plugin-store/signal_tracker.log.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::engine;

/// User-tunable signal tracker parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalTrackerConfig {
    // Signal filter
    pub signal_labels: String,
    pub min_wallet_count: u32,
    pub max_sell_ratio: f64,

    // Safety thresholds
    pub min_mcap: f64,
    pub min_liquidity: f64,
    pub min_holders: i64,
    pub min_liq_mc_ratio: f64,
    pub max_top10_holder_pct: f64,
    pub min_lp_burn: f64,
    pub min_holder_density: f64,
    pub max_k1_pump_pct: f64,

    // Dev/Bundler
    pub dev_max_launched: i64,
    pub dev_max_hold_pct: f64,
    pub bundle_max_ath_pct: f64,
    pub bundle_max_count: i64,

    // Position sizing
    pub position_high_sol: f64,
    pub position_mid_sol: f64,
    pub position_low_sol: f64,
    pub wallet_high_threshold: u32,
    pub wallet_mid_threshold: u32,
    pub max_positions: usize,
    pub slippage_pct: String,
    pub gas_reserve_sol: f64,

    // Cost model
    pub fixed_cost_sol: f64,
    pub cost_per_leg_pct: f64,

    // Take profit (net %)
    pub tp1_pct: f64,
    pub tp1_sell: f64,
    pub tp2_pct: f64,
    pub tp2_sell: f64,
    pub tp3_pct: f64,
    pub tp3_sell: f64,

    // Trailing stop
    pub trail_activate_pct: f64,
    pub trail_distance_pct: f64,

    // Stop loss
    pub sl_multiplier: f64,
    pub liq_emergency: f64,
    pub time_stop_hours: f64,

    // Session risk
    pub max_consec_loss: u32,
    pub pause_consec_sec: u64,
    pub session_loss_limit_sol: f64,
    pub session_loss_pause_sec: u64,
    pub session_stop_sol: f64,

    // Tick
    pub tick_interval_secs: u64,

    // Circuit breaker
    pub max_consecutive_errors: u32,
    pub cooldown_after_errors: u64,
}

impl Default for SignalTrackerConfig {
    fn default() -> Self {
        Self {
            signal_labels: engine::SIGNAL_LABELS.to_string(),
            min_wallet_count: engine::MIN_WALLET_COUNT,
            max_sell_ratio: engine::MAX_SELL_RATIO,
            min_mcap: engine::MIN_MCAP,
            min_liquidity: engine::MIN_LIQUIDITY,
            min_holders: engine::MIN_HOLDERS,
            min_liq_mc_ratio: engine::MIN_LIQ_MC_RATIO,
            max_top10_holder_pct: engine::MAX_TOP10_HOLDER_PCT,
            min_lp_burn: engine::MIN_LP_BURN,
            min_holder_density: engine::MIN_HOLDER_DENSITY,
            max_k1_pump_pct: engine::MAX_K1_PUMP_PCT,
            dev_max_launched: engine::DEV_MAX_LAUNCHED,
            dev_max_hold_pct: engine::DEV_MAX_HOLD_PCT,
            bundle_max_ath_pct: engine::BUNDLE_MAX_ATH_PCT,
            bundle_max_count: engine::BUNDLE_MAX_COUNT,
            position_high_sol: engine::POSITION_HIGH_SOL,
            position_mid_sol: engine::POSITION_MID_SOL,
            position_low_sol: engine::POSITION_LOW_SOL,
            wallet_high_threshold: engine::WALLET_HIGH_THRESHOLD,
            wallet_mid_threshold: engine::WALLET_MID_THRESHOLD,
            max_positions: engine::MAX_POSITIONS,
            slippage_pct: engine::SLIPPAGE_PCT.to_string(),
            gas_reserve_sol: engine::GAS_RESERVE_SOL,
            fixed_cost_sol: engine::FIXED_COST_SOL,
            cost_per_leg_pct: engine::COST_PER_LEG_PCT,
            tp1_pct: engine::TP_TIERS[0].0,
            tp1_sell: engine::TP_TIERS[0].1,
            tp2_pct: engine::TP_TIERS[1].0,
            tp2_sell: engine::TP_TIERS[1].1,
            tp3_pct: engine::TP_TIERS[2].0,
            tp3_sell: engine::TP_TIERS[2].1,
            trail_activate_pct: engine::TRAIL_ACTIVATE_PCT,
            trail_distance_pct: engine::TRAIL_DISTANCE_PCT,
            sl_multiplier: engine::SL_MULTIPLIER,
            liq_emergency: engine::LIQ_EMERGENCY,
            time_stop_hours: engine::TIME_STOP_HOURS,
            max_consec_loss: engine::MAX_CONSEC_LOSS,
            pause_consec_sec: engine::PAUSE_CONSEC_SEC,
            session_loss_limit_sol: engine::SESSION_LOSS_LIMIT_SOL,
            session_loss_pause_sec: engine::SESSION_LOSS_PAUSE_SEC,
            session_stop_sol: engine::SESSION_STOP_SOL,
            tick_interval_secs: engine::TICK_INTERVAL_SECS,
            max_consecutive_errors: engine::MAX_CONSECUTIVE_ERRORS,
            cooldown_after_errors: engine::COOLDOWN_AFTER_ERRORS,
        }
    }
}

impl SignalTrackerConfig {
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".plugin-store")
            .join("signal_tracker_config.json")
    }

    pub fn log_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".plugin-store")
            .join("signal_tracker.log")
    }

    /// Load config from file, falling back to defaults for missing fields.
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: Self = serde_json::from_str(&data)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }

    /// Save config to file.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let dir = path.parent().context("no parent dir")?;
        std::fs::create_dir_all(dir)?;
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, &data)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    /// Calculate breakeven for a given tier.
    pub fn calc_breakeven(&self, sol_amount: f64) -> f64 {
        if sol_amount <= 0.0 {
            return 0.0;
        }
        (self.fixed_cost_sol / sol_amount) * 100.0 + self.cost_per_leg_pct * 2.0
    }
}
