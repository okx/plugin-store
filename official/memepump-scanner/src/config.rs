//! Scanner user-configurable parameters — persisted alongside the executable.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::engine;

/// Base directory for all scanner files (config, state, log, PID).
/// Uses the directory containing the executable, same pattern as grid bot.
pub fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// User-tunable scanner parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    // ── Scan stage ──
    pub stage: String,

    // ── Server-side filters ──
    pub tf_min_mc: u64,
    pub tf_max_mc: u64,
    pub tf_min_holders: u32,
    pub tf_max_dev_hold: u32,
    pub tf_max_bundler: u32,
    pub tf_max_sniper: u32,
    pub tf_max_insider: u32,
    pub tf_max_top10: u32,
    pub tf_max_fresh: u32,
    pub tf_min_tx: u32,
    pub tf_min_buy_tx: u32,
    pub tf_min_age: u32,
    pub tf_max_age: u32,
    pub tf_min_vol: u64,

    // ── Client-side filters ──
    pub cf_min_bs_ratio: f64,
    pub cf_min_vol_mc_pct: f64,
    pub cf_max_top10: f64,

    // ── Deep safety ──
    pub ds_max_dev_hold: f64,
    pub ds_max_bundler_ath: f64,
    pub ds_max_bundler_count: u32,

    // ── Position sizing ──
    pub scalp_sol: f64,
    pub minimum_sol: f64,
    pub max_sol: f64,
    pub max_positions: usize,
    pub slippage_scalp: u32,
    pub slippage_minimum: u32,

    // ── Exit rules ──
    pub tp1_pct: f64,
    pub tp2_pct: f64,
    pub sl_scalp: f64,
    pub sl_hot: f64,
    pub sl_quiet: f64,
    pub trailing_pct: f64,
    pub max_hold_min: u64,

    // ── Session risk ──
    pub max_consec_loss: u32,
    pub pause_loss_sol: f64,
    pub stop_loss_sol: f64,

    // ── Tick ──
    pub tick_interval_secs: u64,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            stage: "MIGRATED".to_string(),
            tf_min_mc: engine::TF_MIN_MC,
            tf_max_mc: engine::TF_MAX_MC,
            tf_min_holders: engine::TF_MIN_HOLDERS,
            tf_max_dev_hold: engine::TF_MAX_DEV_HOLD,
            tf_max_bundler: engine::TF_MAX_BUNDLER,
            tf_max_sniper: engine::TF_MAX_SNIPER,
            tf_max_insider: engine::TF_MAX_INSIDER,
            tf_max_top10: engine::TF_MAX_TOP10,
            tf_max_fresh: engine::TF_MAX_FRESH,
            tf_min_tx: engine::TF_MIN_TX,
            tf_min_buy_tx: engine::TF_MIN_BUY_TX,
            tf_min_age: engine::TF_MIN_AGE,
            tf_max_age: engine::TF_MAX_AGE,
            tf_min_vol: engine::TF_MIN_VOL,
            cf_min_bs_ratio: engine::CF_MIN_BS_RATIO,
            cf_min_vol_mc_pct: engine::CF_MIN_VOL_MC_PCT,
            cf_max_top10: engine::CF_MAX_TOP10,
            ds_max_dev_hold: engine::DS_MAX_DEV_HOLD,
            ds_max_bundler_ath: engine::DS_MAX_BUNDLER_ATH,
            ds_max_bundler_count: engine::DS_MAX_BUNDLER_COUNT,
            scalp_sol: engine::SCALP_SOL,
            minimum_sol: engine::MINIMUM_SOL,
            max_sol: engine::MAX_SOL,
            max_positions: engine::MAX_POSITIONS,
            slippage_scalp: engine::SLIPPAGE_SCALP,
            slippage_minimum: engine::SLIPPAGE_MINIMUM,
            tp1_pct: engine::TP1_PCT,
            tp2_pct: engine::TP2_PCT,
            sl_scalp: engine::SL_SCALP,
            sl_hot: engine::SL_HOT,
            sl_quiet: engine::SL_QUIET,
            trailing_pct: engine::TRAILING_PCT,
            max_hold_min: engine::MAX_HOLD_MIN,
            max_consec_loss: engine::MAX_CONSEC_LOSS,
            pause_loss_sol: engine::PAUSE_LOSS_SOL,
            stop_loss_sol: engine::STOP_LOSS_SOL,
            tick_interval_secs: engine::TICK_INTERVAL_SECS,
        }
    }
}

impl ScannerConfig {
    pub fn config_path() -> PathBuf {
        base_dir().join("memepump_scanner_config.json")
    }

    /// Log file path.
    pub fn log_path() -> PathBuf {
        base_dir().join("memepump_scanner.log")
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

    /// Position size for a given tier.
    pub fn position_size(&self, tier: engine::SignalTier) -> f64 {
        match tier {
            engine::SignalTier::Scalp => self.scalp_sol,
            engine::SignalTier::Minimum => self.minimum_sol,
        }
    }

    /// Slippage for a given tier.
    pub fn slippage(&self, tier: engine::SignalTier) -> u32 {
        match tier {
            engine::SignalTier::Scalp => self.slippage_scalp,
            engine::SignalTier::Minimum => self.slippage_minimum,
        }
    }

    /// Build ExitParams from config.
    pub fn exit_params(&self) -> engine::ExitParams {
        engine::ExitParams {
            tp1_pct: self.tp1_pct,
            tp2_pct: self.tp2_pct,
            sl_scalp: self.sl_scalp,
            sl_hot: self.sl_hot,
            sl_quiet: self.sl_quiet,
            trailing_pct: self.trailing_pct,
            max_hold_min: self.max_hold_min,
        }
    }

    /// Calculate breakeven percentage for a given SOL amount.
    pub fn calc_breakeven(&self, sol_amount: f64) -> f64 {
        engine::calc_breakeven_pct(sol_amount)
    }
}
