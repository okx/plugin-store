//! Signal tracker state management — persisted at ~/.plugin-store/signal_tracker_state.json.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::engine::{
    Position, Trade, COOLDOWN_AFTER_ERRORS, MAX_CONSECUTIVE_ERRORS, MAX_KNOWN_TOKENS, MAX_TRADES,
};

const STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub total_buys: u32,
    pub total_sells: u32,
    pub successful_trades: u32,
    pub failed_trades: u32,
    pub total_invested_sol: f64,
    pub total_returned_sol: f64,
    pub session_pnl_sol: f64,
    pub consecutive_losses: u32,
    pub cumulative_loss_sol: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorState {
    pub consecutive_errors: u32,
    pub last_error_time: Option<String>,
    pub last_error_msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalTrackerState {
    pub version: u32,
    pub known_tokens: HashSet<String>,
    pub positions: HashMap<String, Position>,
    pub trades: Vec<Trade>,
    pub stats: Stats,
    pub errors: ErrorState,
    pub paused_until: Option<i64>,
    pub stopped: bool,
    pub stop_reason: Option<String>,
    pub dry_run: bool,
}

impl Default for SignalTrackerState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            known_tokens: HashSet::new(),
            positions: HashMap::new(),
            trades: Vec::new(),
            stats: Stats::default(),
            errors: ErrorState::default(),
            paused_until: None,
            stopped: false,
            stop_reason: None,
            dry_run: false,
        }
    }
}

impl SignalTrackerState {
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
            .join("signal_tracker_state.json")
    }

    pub fn pid_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".plugin-store")
            .join("signal_tracker.pid")
    }

    pub fn push_trade(&mut self, trade: Trade) {
        self.trades.push(trade);
        if self.trades.len() > MAX_TRADES {
            let excess = self.trades.len() - MAX_TRADES;
            self.trades.drain(..excess);
        }
    }

    /// Trim known_tokens to prevent unbounded growth.
    pub fn trim_known_tokens(&mut self) {
        if self.known_tokens.len() > MAX_KNOWN_TOKENS {
            let to_remove = self.known_tokens.len() - MAX_KNOWN_TOKENS / 2;
            let remove_list: Vec<String> =
                self.known_tokens.iter().take(to_remove).cloned().collect();
            for k in remove_list {
                self.known_tokens.remove(&k);
            }
        }
    }

    pub fn check_circuit_breaker(&self) -> Option<String> {
        if self.errors.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
            if let Some(ref last_err_time) = self.errors.last_error_time {
                if let Ok(t) = chrono::DateTime::parse_from_rfc3339(last_err_time) {
                    let elapsed = chrono::Utc::now().signed_duration_since(t).num_seconds() as u64;
                    if elapsed < COOLDOWN_AFTER_ERRORS {
                        return Some(format!(
                            "Circuit breaker: {} consecutive errors, cooldown {}s remaining",
                            self.errors.consecutive_errors,
                            COOLDOWN_AFTER_ERRORS - elapsed
                        ));
                    }
                }
            }
        }
        None
    }

    /// Check if paused. Returns true if still paused.
    pub fn is_paused(&self) -> bool {
        if let Some(until) = self.paused_until {
            chrono::Utc::now().timestamp() < until
        } else {
            false
        }
    }

    /// Record a loss and update session risk counters.
    pub fn record_loss(&mut self, loss_sol: f64) {
        self.stats.consecutive_losses += 1;
        self.stats.cumulative_loss_sol += loss_sol.abs();
    }

    /// Record a win and reset consecutive loss counter.
    pub fn record_win(&mut self) {
        self.stats.consecutive_losses = 0;
    }
}
