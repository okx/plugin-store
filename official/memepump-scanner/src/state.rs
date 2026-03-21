//! Memepump scanner state management — persisted at ~/.plugin-store/memepump_scanner_state.json.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::engine::{
    LaunchType, SignalTier, ERROR_COOLDOWN_SEC, MAX_CONSEC_ERRORS, MAX_PREV_TX, MAX_SIGNALS,
    MAX_TRADES,
};

const STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub token_address: String,
    pub symbol: String,
    pub tier: SignalTier,
    pub launch: LaunchType,
    pub entry_price: f64,
    pub entry_sol: f64,
    pub token_amount_raw: String,
    pub entry_time: String,
    pub peak_price: f64,
    pub tp1_hit: bool,
    pub breakeven_pct: f64,
    pub sell_fail_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub time: String,
    pub token_address: String,
    pub symbol: String,
    pub direction: String,
    pub sol_amount: f64,
    pub price: f64,
    pub tier: SignalTier,
    pub launch: LaunchType,
    pub tx_hash: Option<String>,
    pub success: bool,
    pub exit_reason: Option<String>,
    pub pnl_sol: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRecord {
    pub time: String,
    pub token_address: String,
    pub symbol: String,
    pub tier: SignalTier,
    pub launch: LaunchType,
    pub sig_a_ratio: f64,
    pub sig_b_ratio: f64,
    pub market_cap: f64,
    pub acted: bool,
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStats {
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
pub struct ScannerState {
    pub version: u32,
    pub prev_tx: HashSet<String>,
    pub positions: HashMap<String, Position>,
    pub trades: Vec<Trade>,
    pub signals: Vec<SignalRecord>,
    pub stats: SessionStats,
    pub errors: ErrorState,
    pub paused_until: Option<String>,
    pub stopped: bool,
    pub stop_reason: Option<String>,
    pub dry_run: bool,
}

impl Default for ScannerState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            prev_tx: HashSet::new(),
            positions: HashMap::new(),
            trades: Vec::new(),
            signals: Vec::new(),
            stats: SessionStats::default(),
            errors: ErrorState::default(),
            paused_until: None,
            stopped: false,
            stop_reason: None,
            dry_run: false,
        }
    }
}

impl ScannerState {
    /// Load state from ~/.plugin-store/memepump_scanner_state.json.
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
        super::config::base_dir().join("memepump_scanner_state.json")
    }

    pub fn pid_path() -> PathBuf {
        super::config::base_dir().join("memepump_scanner.pid")
    }

    /// Add a trade, trimming to MAX_TRADES.
    pub fn push_trade(&mut self, trade: Trade) {
        self.trades.push(trade);
        if self.trades.len() > MAX_TRADES {
            let excess = self.trades.len() - MAX_TRADES;
            self.trades.drain(..excess);
        }
    }

    /// Add a signal record, trimming to MAX_SIGNALS.
    pub fn push_signal(&mut self, signal: SignalRecord) {
        self.signals.push(signal);
        if self.signals.len() > MAX_SIGNALS {
            let excess = self.signals.len() - MAX_SIGNALS;
            self.signals.drain(..excess);
        }
    }

    /// Trim prev_tx to prevent unbounded growth.
    pub fn trim_prev_tx(&mut self) {
        if self.prev_tx.len() > MAX_PREV_TX {
            let to_remove = self.prev_tx.len() - MAX_PREV_TX / 2;
            let remove_list: Vec<String> = self.prev_tx.iter().take(to_remove).cloned().collect();
            for k in remove_list {
                self.prev_tx.remove(&k);
            }
        }
    }

    /// Check circuit breaker.
    pub fn check_circuit_breaker(&self) -> Option<String> {
        if self.errors.consecutive_errors >= MAX_CONSEC_ERRORS {
            if let Some(ref last_err_time) = self.errors.last_error_time {
                if let Ok(t) = chrono::DateTime::parse_from_rfc3339(last_err_time) {
                    let elapsed = chrono::Utc::now().signed_duration_since(t).num_seconds() as u64;
                    if elapsed < ERROR_COOLDOWN_SEC {
                        return Some(format!(
                            "Circuit breaker: {} consecutive errors, cooldown {}s remaining",
                            self.errors.consecutive_errors,
                            ERROR_COOLDOWN_SEC - elapsed
                        ));
                    }
                }
            }
        }
        None
    }

    /// Check if paused. Returns true if still paused.
    pub fn is_paused(&self) -> bool {
        if let Some(ref until) = self.paused_until {
            if let Ok(t) = chrono::DateTime::parse_from_rfc3339(until) {
                return chrono::Utc::now() < t;
            }
        }
        false
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
