//! Ranking sniper state management — persisted at ~/.plugin-store/ranking_sniper_state.json.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::config::SniperConfig;
use super::engine::{Position, Trade, MAX_TRADES};

const STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub total_buys: u32,
    pub total_sells: u32,
    pub successful_trades: u32,
    pub failed_trades: u32,
    pub total_invested_sol: f64,
    pub total_returned_sol: f64,
    pub daily_pnl_sol: f64,
    pub daily_start_time: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorState {
    pub consecutive_errors: u32,
    pub last_error_time: Option<String>,
    pub last_error_msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperState {
    pub version: u32,
    #[serde(default)]
    pub config: SniperConfig,
    pub known_tokens: HashSet<String>,
    pub positions: HashMap<String, Position>, // token_address -> Position
    pub trades: Vec<Trade>,
    pub stats: Stats,
    pub errors: ErrorState,
    pub remaining_budget_sol: f64,
    pub stopped: bool,
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub last_sell_times: HashMap<String, String>, // token_address -> RFC3339 timestamp of last sell
}

impl Default for SniperState {
    fn default() -> Self {
        let config = SniperConfig::default();
        let budget = config.budget_sol;
        Self {
            version: STATE_VERSION,
            config,
            known_tokens: HashSet::new(),
            positions: HashMap::new(),
            trades: Vec::new(),
            stats: Stats::default(),
            errors: ErrorState::default(),
            remaining_budget_sol: budget,
            stopped: false,
            stop_reason: None,
            last_sell_times: HashMap::new(),
        }
    }
}

impl SniperState {
    /// Load state from ~/.plugin-store/ranking_sniper_state.json.
    pub fn load() -> Result<Self> {
        let path = Self::state_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let state: Self = serde_json::from_str(&data)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(state)
    }

    /// Save state atomically: write to .tmp file, then rename.
    pub fn save(&self) -> Result<()> {
        let path = Self::state_path();
        let dir = path.parent().context("no parent dir")?;
        std::fs::create_dir_all(dir)?;

        let tmp_path = path.with_extension("json.tmp");
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp_path, &data)
            .with_context(|| format!("failed to write {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &path).with_context(|| {
            format!(
                "failed to rename {} -> {}",
                tmp_path.display(),
                path.display()
            )
        })?;
        Ok(())
    }

    /// Delete state file.
    pub fn reset() -> Result<()> {
        let path = Self::state_path();
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub fn state_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".plugin-store")
            .join("ranking_sniper_state.json")
    }

    pub fn pid_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".plugin-store")
            .join("ranking_sniper.pid")
    }

    /// Add a trade, trimming to MAX_TRADES.
    pub fn push_trade(&mut self, trade: Trade) {
        self.trades.push(trade);
        if self.trades.len() > MAX_TRADES {
            let excess = self.trades.len() - MAX_TRADES;
            self.trades.drain(..excess);
        }
    }

    /// Check circuit breaker using config thresholds.
    pub fn check_circuit_breaker(&self, cfg: &SniperConfig) -> Option<String> {
        if self.errors.consecutive_errors >= cfg.max_consecutive_errors {
            if let Some(ref last_err_time) = self.errors.last_error_time {
                if let Ok(t) = chrono::DateTime::parse_from_rfc3339(last_err_time) {
                    let elapsed = chrono::Utc::now().signed_duration_since(t).num_seconds() as u64;
                    if elapsed < cfg.cooldown_after_errors {
                        return Some(format!(
                            "Circuit breaker: {} consecutive errors, cooldown {}s remaining",
                            self.errors.consecutive_errors,
                            cfg.cooldown_after_errors - elapsed
                        ));
                    }
                }
            }
        }
        None
    }

    /// Check if a token is still in cooldown period after being sold.
    pub fn is_cooldown_active(&self, token_address: &str, cfg: &SniperConfig) -> bool {
        if let Some(sell_time) = self.last_sell_times.get(token_address) {
            if let Ok(t) = chrono::DateTime::parse_from_rfc3339(sell_time) {
                let elapsed = chrono::Utc::now().signed_duration_since(t).num_minutes();
                return elapsed < cfg.cooldown_minutes as i64;
            }
        }
        false
    }

    /// Record the time of a sell for cooldown tracking.
    pub fn record_sell_time(&mut self, token_address: &str) {
        self.last_sell_times
            .insert(token_address.to_string(), chrono::Utc::now().to_rfc3339());
    }

    /// Reset daily PnL tracking if a new day has started.
    pub fn maybe_reset_daily(&mut self) {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        match &self.stats.daily_start_time {
            Some(d) if d == &today => {}
            _ => {
                self.stats.daily_pnl_sol = 0.0;
                self.stats.daily_start_time = Some(today);
            }
        }
    }
}
