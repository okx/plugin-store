//! Ranking sniper user-configurable parameters — persisted at ~/.plugin-store/ranking_sniper_config.json.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::engine;

/// User-tunable ranking sniper parameters. Loaded from config file with defaults from engine constants.
/// `#[serde(default)]` ensures backward compatibility: old state files with fewer fields still parse.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SniperConfig {
    // ── 资金管理 ──
    pub budget_sol: f64,
    pub per_trade_sol: f64,
    pub max_positions: usize,
    pub gas_reserve_sol: f64,
    pub min_wallet_balance: f64,
    pub daily_loss_limit_pct: f64,
    pub dry_run: bool,

    // ── 交易参数 ──
    pub slippage_pct: String,
    pub score_buy_threshold: u32,
    pub tick_interval_secs: u64,
    pub cooldown_minutes: u64,
    pub top_n: usize,

    // ── 一级 Slot Guard ──
    pub min_change_pct: f64,
    pub max_change_pct: f64,
    pub min_liquidity: f64,
    pub min_market_cap: f64,
    pub max_market_cap: f64,
    pub min_holders: i64,
    pub min_buy_ratio: f64,
    pub min_traders: i64,

    // ── 二级 Advanced Safety ──
    pub max_risk_level: i64,
    pub max_top10_hold: f64,
    pub max_dev_hold: f64,
    pub max_bundler_hold: f64,
    pub min_lp_burn: f64,
    pub max_dev_rug_count: i64,
    pub max_sniper_hold: f64,
    pub block_internal: bool,

    // ── 三级 Holder Risk Scan ──
    pub max_suspicious_hold: f64,
    pub max_suspicious_count: usize,
    pub block_phishing: bool,

    // ── 退出系统 ──
    pub hard_stop_pct: f64,
    pub fast_stop_time_secs: u64,
    pub fast_stop_pct: f64,
    pub trailing_activate_pct: f64,
    pub trailing_drawdown_pct: f64,
    pub time_stop_secs: u64,
    pub tp_levels: [f64; 3],

    // ── 熔断 ──
    pub max_consecutive_errors: u32,
    pub cooldown_after_errors: u64,

    // ── 日志 ──
    pub log_file: Option<String>,

    // ── Telegram 通知 ──
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
}

impl Default for SniperConfig {
    fn default() -> Self {
        Self {
            // 资金管理
            budget_sol: engine::DEFAULT_BUDGET_SOL,
            per_trade_sol: engine::DEFAULT_PER_TRADE_SOL,
            max_positions: engine::MAX_POSITIONS,
            gas_reserve_sol: engine::GAS_RESERVE_SOL,
            min_wallet_balance: engine::MIN_WALLET_BALANCE,
            daily_loss_limit_pct: engine::DAILY_LOSS_LIMIT_PCT,
            dry_run: false,

            // 交易参数
            slippage_pct: engine::SLIPPAGE_PCT.to_string(),
            score_buy_threshold: engine::SCORE_BUY_THRESHOLD,
            tick_interval_secs: engine::TICK_INTERVAL_SECS,
            cooldown_minutes: engine::COOLDOWN_MINUTES,
            top_n: engine::TOP_N,

            // 一级 Slot Guard
            min_change_pct: engine::MIN_CHANGE_PCT,
            max_change_pct: engine::MAX_CHANGE_PCT,
            min_liquidity: engine::MIN_LIQUIDITY,
            min_market_cap: engine::MIN_MARKET_CAP,
            max_market_cap: engine::MAX_MARKET_CAP,
            min_holders: engine::MIN_HOLDERS,
            min_buy_ratio: engine::MIN_BUY_RATIO,
            min_traders: engine::MIN_TRADERS,

            // 二级 Advanced Safety
            max_risk_level: engine::MAX_RISK_LEVEL,
            max_top10_hold: engine::MAX_TOP10_HOLD,
            max_dev_hold: engine::MAX_DEV_HOLD,
            max_bundler_hold: engine::MAX_BUNDLER_HOLD,
            min_lp_burn: engine::MIN_LP_BURN,
            max_dev_rug_count: engine::MAX_DEV_RUG_COUNT,
            max_sniper_hold: engine::MAX_SNIPER_HOLD,
            block_internal: engine::BLOCK_INTERNAL,

            // 三级 Holder Risk Scan
            max_suspicious_hold: engine::MAX_SUSPICIOUS_HOLD,
            max_suspicious_count: engine::MAX_SUSPICIOUS_COUNT,
            block_phishing: engine::BLOCK_PHISHING,

            // 退出系统
            hard_stop_pct: engine::HARD_STOP_PCT,
            fast_stop_time_secs: engine::FAST_STOP_TIME_SECS,
            fast_stop_pct: engine::FAST_STOP_PCT,
            trailing_activate_pct: engine::TRAILING_ACTIVATE_PCT,
            trailing_drawdown_pct: engine::TRAILING_DRAWDOWN_PCT,
            time_stop_secs: engine::TIME_STOP_SECS,
            tp_levels: engine::TP_LEVELS,

            // 熔断
            max_consecutive_errors: engine::MAX_CONSECUTIVE_ERRORS,
            cooldown_after_errors: engine::COOLDOWN_AFTER_ERRORS,

            // 日志
            log_file: None,

            // Telegram 通知
            telegram_bot_token: None,
            telegram_chat_id: None,
        }
    }
}

impl SniperConfig {
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".plugin-store")
            .join("ranking_sniper_config.json")
    }

    pub fn default_log_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".plugin-store")
            .join("ranking_sniper.log")
    }

    /// Resolved log file path: user override or default.
    pub fn log_path(&self) -> PathBuf {
        match &self.log_file {
            Some(p) => PathBuf::from(p),
            None => Self::default_log_path(),
        }
    }

    /// Load config from file, falling back to defaults if file does not exist.
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

    /// Save config to file (pretty JSON).
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        let dir = path.parent().context("no parent dir")?;
        std::fs::create_dir_all(dir)?;
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, &data)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    /// Print parameter summary to stderr.
    pub fn print_summary(&self) {
        eprintln!();
        eprintln!("========== SOL Ranking Sniper - 当前参数 ==========");
        eprintln!();
        eprintln!("[资金管理]");
        eprintln!("  总预算:         {} SOL", self.budget_sol);
        eprintln!("  单笔买入:       {} SOL", self.per_trade_sol);
        eprintln!("  最大持仓:       {}", self.max_positions);
        eprintln!("  日亏损上限:     {}%", self.daily_loss_limit_pct);
        eprintln!("  模拟模式:       {}", self.dry_run);
        eprintln!();
        eprintln!("[交易参数]");
        eprintln!("  滑点:           {}%", self.slippage_pct);
        eprintln!("  评分阈值:       {}/125", self.score_buy_threshold);
        eprintln!("  轮询间隔:       {}s", self.tick_interval_secs);
        eprintln!("  冷却期:         {}min", self.cooldown_minutes);
        eprintln!("  排行榜 Top:     {}", self.top_n);
        eprintln!();
        eprintln!("[一级 Slot Guard]");
        eprintln!(
            "  涨幅:           {}% ~ {}%",
            self.min_change_pct, self.max_change_pct
        );
        eprintln!("  流动性:         >= ${}", self.min_liquidity);
        eprintln!(
            "  市值:           ${} ~ ${}M",
            self.min_market_cap,
            self.max_market_cap / 1e6
        );
        eprintln!("  持有者:         >= {}", self.min_holders);
        eprintln!("  买入比:         >= {}%", self.min_buy_ratio * 100.0);
        eprintln!("  独立交易者:     >= {}", self.min_traders);
        eprintln!();
        eprintln!("[二级 Advanced Safety]");
        eprintln!("  风控等级:       <= {}", self.max_risk_level);
        eprintln!("  Top10持仓:      <= {}%", self.max_top10_hold);
        eprintln!("  Dev持仓:        <= {}%", self.max_dev_hold);
        eprintln!("  Bundler持仓:    <= {}%", self.max_bundler_hold);
        eprintln!("  LP销毁:         >= {}%", self.min_lp_burn);
        eprintln!("  DevRug历史:     <= {}", self.max_dev_rug_count);
        eprintln!("  狙击手持仓:     <= {}%", self.max_sniper_hold);
        eprintln!("  拒绝内盘:       {}", self.block_internal);
        eprintln!();
        eprintln!("[三级 Holder Risk Scan]");
        eprintln!("  可疑持仓:       <= {}%", self.max_suspicious_hold);
        eprintln!("  可疑地址数:     <= {}", self.max_suspicious_count);
        eprintln!("  拒绝钓鱼:       {}", self.block_phishing);
        eprintln!();
        eprintln!("[退出系统]");
        eprintln!("  硬止损:         {}%", self.hard_stop_pct);
        eprintln!(
            "  快速止损:       {}min / {}%",
            self.fast_stop_time_secs / 60,
            self.fast_stop_pct
        );
        eprintln!(
            "  追踪止损:       +{}% 激活 / {}% 回撤",
            self.trailing_activate_pct, self.trailing_drawdown_pct
        );
        eprintln!(
            "  时间止损:       {}s ({})",
            self.time_stop_secs,
            if self.time_stop_secs < 3600 {
                "测试值"
            } else {
                "生产值"
            }
        );
        eprintln!(
            "  梯度止盈:       +{}% / +{}% / +{}%",
            self.tp_levels[0], self.tp_levels[1], self.tp_levels[2]
        );
        eprintln!();
        eprintln!("[日志]");
        eprintln!("  日志文件:       {}", self.log_path().display());
        eprintln!();
        eprintln!("[Telegram 通知]");
        let tg_status = if self.telegram_bot_token.is_some() && self.telegram_chat_id.is_some() {
            "已配置"
        } else {
            "未配置 (设置 telegram_bot_token + telegram_chat_id 或环境变量 TELEGRAM_BOT_TOKEN + TELEGRAM_CHAT_ID)"
        };
        eprintln!("  状态:           {}", tg_status);
        eprintln!();
        eprintln!("  配置文件:       {}", Self::config_path().display());
        eprintln!("==================================================");
        eprintln!();
    }
}
