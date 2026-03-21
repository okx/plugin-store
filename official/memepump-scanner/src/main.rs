use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use memepump_scanner::client::ScannerClient;
use memepump_scanner::config::ScannerConfig;
use memepump_scanner::engine::{
    self, calc_breakeven_pct, check_exit, check_session_risk, classify_launch, classify_token,
    deep_safety_check_with, detect_signal, exit_sell_pct, safe_float, safe_u32,
    LaunchType, SignalTier, TokenData,
};
use memepump_scanner::state::{Position, ScannerState, SignalRecord, Trade};

#[derive(Parser)]
#[command(
    name = "strategy-memepump-scanner",
    version,
    about = "Memepump Scanner — automated Solana memepump token trading"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute one tick cycle
    Tick {
        #[arg(long)]
        dry_run: bool,
    },
    /// Start continuous bot
    Start {
        #[arg(long)]
        dry_run: bool,
    },
    /// Stop running bot
    Stop,
    /// Show state, positions, PnL
    Status,
    /// Detailed PnL and performance stats
    Report,
    /// Trade history
    History {
        #[arg(long, default_value = "50")]
        limit: usize,
    },
    /// Analyze current memepump market
    Analyze,
    /// Show all configurable parameters
    Config,
    /// Force-sell all open positions
    SellAll,
    /// Sell specific token
    Sell {
        token_address: String,
        #[arg(long)]
        amount: String,
    },
    /// Buy+sell round-trip (debug)
    TestTrade {
        token_address: String,
        #[arg(long, default_value = "0.01")]
        amount: f64,
    },
    /// Clear all state data
    Reset {
        #[arg(long)]
        force: bool,
    },
    /// Check wallet balance sufficiency
    Balance,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = ScannerConfig::load()?;

    match cli.command {
        Commands::Analyze => cmd_analyze(&config).await,
        Commands::Config => {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "stage": config.stage,
                "server_filters": {
                    "min_mc": config.tf_min_mc,
                    "max_mc": config.tf_max_mc,
                    "min_holders": config.tf_min_holders,
                    "max_dev_hold_pct": config.tf_max_dev_hold,
                    "max_bundler_pct": config.tf_max_bundler,
                    "max_sniper_pct": config.tf_max_sniper,
                    "max_insider_pct": config.tf_max_insider,
                    "max_top10_pct": config.tf_max_top10,
                    "max_fresh_wallet_pct": config.tf_max_fresh,
                    "min_tx": config.tf_min_tx,
                    "min_buy_tx": config.tf_min_buy_tx,
                    "min_age_min": config.tf_min_age,
                    "max_age_min": config.tf_max_age,
                    "min_vol_usd": config.tf_min_vol,
                },
                "client_filters": {
                    "min_bs_ratio": config.cf_min_bs_ratio,
                    "min_vol_mc_pct": config.cf_min_vol_mc_pct,
                    "max_top10_pct": config.cf_max_top10,
                },
                "position_sizing": {
                    "scalp_sol": config.scalp_sol,
                    "minimum_sol": config.minimum_sol,
                    "max_sol": config.max_sol,
                    "max_positions": config.max_positions,
                    "slippage_scalp_pct": config.slippage_scalp,
                    "slippage_minimum_pct": config.slippage_minimum,
                },
                "exit_rules": {
                    "tp1_pct": config.tp1_pct,
                    "tp2_pct": config.tp2_pct,
                    "sl_scalp_pct": config.sl_scalp,
                    "sl_hot_pct": config.sl_hot,
                    "sl_quiet_pct": config.sl_quiet,
                    "trailing_pct": config.trailing_pct,
                    "max_hold_min": config.max_hold_min,
                },
                "session_risk": {
                    "max_consec_loss": config.max_consec_loss,
                    "pause_loss_sol": config.pause_loss_sol,
                    "stop_loss_sol": config.stop_loss_sol,
                },
                "tick_interval_secs": config.tick_interval_secs,
            }))?);
            Ok(())
        }
        Commands::Status => cmd_status(),
        Commands::Report => cmd_report(),
        Commands::History { limit } => cmd_history(limit),
        Commands::Balance => cmd_balance(&config).await,
        Commands::Tick { dry_run } => {
            if dry_run {
                config.stage = config.stage.clone();
            }
            cmd_tick(&config, dry_run).await
        }
        Commands::Start { dry_run } => cmd_start(&config, dry_run).await,
        Commands::Stop => cmd_stop(),
        Commands::SellAll => cmd_sell_all().await,
        Commands::Sell {
            token_address,
            amount,
        } => cmd_sell(&token_address, &amount).await,
        Commands::TestTrade {
            token_address,
            amount,
        } => cmd_test_trade(&token_address, amount).await,
        Commands::Reset { force } => cmd_reset(force),
    }
}

// ── Helpers ────────────────────────────────────────────────────────

/// Parse TokenData from the API response JSON.
fn parse_token_data(t: &serde_json::Value) -> Option<TokenData> {
    let addr = t["tokenContractAddress"].as_str()?.to_string();
    let symbol = t["tokenSymbol"].as_str().unwrap_or("?").to_string();
    let name = t["tokenName"].as_str().unwrap_or(&symbol).to_string();

    let market_cap = safe_float(&t["marketCap"], safe_float(&t["marketCapUsd"], 0.0));
    let volume_1h = safe_float(&t["volumeUsd"], safe_float(&t["volume1h"], 0.0));
    let buy_tx_1h = safe_u32(&t["buyTxCount1h"], safe_u32(&t["buyTxCount"], 0));
    let sell_tx_1h = safe_u32(&t["sellTxCount1h"], safe_u32(&t["sellTxCount"], 0));
    let holders = safe_u32(&t["holdersCount"], safe_u32(&t["holders"], 0));
    let top10_pct = safe_float(&t["top10HoldingsPercent"], safe_float(&t["top10Percent"], 100.0));
    let dev_hold_pct = safe_float(&t["devHoldingsPercent"], safe_float(&t["devHoldPercent"], 0.0));
    let bundler_pct = safe_float(&t["bundlersPercent"], safe_float(&t["bundlerPercent"], 0.0));
    let sniper_pct = safe_float(&t["snipersPercent"], safe_float(&t["sniperPercent"], 0.0));
    let insider_pct = safe_float(&t["insidersPercent"], safe_float(&t["insiderPercent"], 0.0));
    let fresh_wallet_pct =
        safe_float(&t["freshWalletPercent"], safe_float(&t["freshPercent"], 0.0));
    let created_timestamp = t["createdTimestamp"]
        .as_u64()
        .or_else(|| t["createdTimestamp"].as_str()?.parse().ok())
        .unwrap_or(0);

    Some(TokenData {
        token_address: addr,
        symbol,
        name,
        market_cap,
        volume_1h,
        buy_tx_1h,
        sell_tx_1h,
        holders,
        top10_pct,
        dev_hold_pct,
        bundler_pct,
        sniper_pct,
        insider_pct,
        fresh_wallet_pct,
        created_timestamp,
    })
}

/// Extract (current_vol, prev_5_vols) from candle data (OKX 1m format).
/// Candle array: [ts, open, high, low, close, vol, volCcy, ...]
fn extract_candle_vols(candles: &serde_json::Value) -> (f64, Vec<f64>) {
    let arr = match candles.as_array() {
        Some(a) if a.len() >= 2 => a,
        _ => return (0.0, vec![]),
    };

    fn parse_vol(c: &serde_json::Value) -> f64 {
        c.as_array()
            .and_then(|row| row.get(5))
            .map(|v| safe_float(v, 0.0))
            .unwrap_or(0.0)
    }

    let current = parse_vol(arr.last().unwrap());
    let prev: Vec<f64> = arr[..arr.len().saturating_sub(1)]
        .iter()
        .map(parse_vol)
        .collect();

    (current, prev)
}

/// Compute sell amount raw string, with remaining.
fn split_sell_raw(amount_raw: &str, sell_pct: f64) -> (String, String) {
    let total: u64 = amount_raw.parse().unwrap_or(0);
    let sell = ((total as f64 * sell_pct).round() as u64).min(total);
    let remaining = total - sell;
    (sell.to_string(), remaining.to_string())
}

/// Build API params JSON from config.
fn build_api_params(config: &ScannerConfig) -> serde_json::Value {
    serde_json::json!({
        "chainIndex": engine::CHAIN_INDEX,
        "stage": config.stage,
        "minMarketCapUsd": config.tf_min_mc.to_string(),
        "maxMarketCapUsd": config.tf_max_mc.to_string(),
        "minHolders": config.tf_min_holders.to_string(),
        "maxDevHoldingsPercent": config.tf_max_dev_hold.to_string(),
        "maxBundlersPercent": config.tf_max_bundler.to_string(),
        "maxSnipersPercent": config.tf_max_sniper.to_string(),
        "maxInsidersPercent": config.tf_max_insider.to_string(),
        "maxTop10HoldingsPercent": config.tf_max_top10.to_string(),
        "maxFreshWalletPercent": config.tf_max_fresh.to_string(),
        "minTxCount": config.tf_min_tx.to_string(),
        "minBuyTxCount": config.tf_min_buy_tx.to_string(),
        "minTokenAge": config.tf_min_age.to_string(),
        "maxTokenAge": config.tf_max_age.to_string(),
        "minVolume": config.tf_min_vol.to_string(),
    })
}

fn check_pid_file() -> bool {
    let pid_path = ScannerState::pid_path();
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            #[cfg(unix)]
            unsafe {
                return libc::kill(pid, 0) == 0;
            }
        }
    }
    false
}

// ── Commands ────────────────────────────────────────────────────────

async fn cmd_analyze(config: &ScannerConfig) -> Result<()> {
    let client = ScannerClient::new_read_only()?;
    let params = build_api_params(config);
    let tokens = client.get_memepump_list(&params).await?;

    let summary: Vec<_> = tokens
        .iter()
        .take(20)
        .filter_map(|t| parse_token_data(t))
        .map(|td| {
            serde_json::json!({
                "symbol": td.symbol,
                "address": td.token_address,
                "market_cap": format!("${:.0}", td.market_cap),
                "volume_1h": format!("${:.0}", td.volume_1h),
                "holders": td.holders,
                "buy_tx": td.buy_tx_1h,
                "sell_tx": td.sell_tx_1h,
                "top10_pct": format!("{:.1}%", td.top10_pct),
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "total_tokens": tokens.len(),
            "top_tokens": summary,
        }))?
    );
    Ok(())
}

fn cmd_status() -> Result<()> {
    let state = ScannerState::load()?;
    let pid_running = check_pid_file();

    let output = serde_json::json!({
        "bot_running": pid_running,
        "stopped": state.stopped,
        "stop_reason": state.stop_reason,
        "dry_run": state.dry_run,
        "position_count": state.positions.len(),
        "positions": state.positions.iter().map(|(addr, pos)| {
            serde_json::json!({
                "token": addr,
                "symbol": pos.symbol,
                "tier": format!("{:?}", pos.tier),
                "launch": format!("{:?}", pos.launch),
                "entry_price": pos.entry_price,
                "entry_sol": pos.entry_sol,
                "tp1_hit": pos.tp1_hit,
            })
        }).collect::<Vec<_>>(),
        "session_pnl_sol": state.stats.session_pnl_sol,
        "total_buys": state.stats.total_buys,
        "total_sells": state.stats.total_sells,
        "consecutive_losses": state.stats.consecutive_losses,
        "paused_until": state.paused_until,
        "consecutive_errors": state.errors.consecutive_errors,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn cmd_report() -> Result<()> {
    let state = ScannerState::load()?;

    let sell_trades: Vec<_> = state
        .trades
        .iter()
        .filter(|t| t.direction == "SELL" && t.success)
        .collect();
    let wins = sell_trades
        .iter()
        .filter(|t| t.pnl_sol.unwrap_or(0.0) > 0.0)
        .count();
    let losses = sell_trades.len() - wins;
    let win_rate = if !sell_trades.is_empty() {
        wins as f64 / sell_trades.len() as f64 * 100.0
    } else {
        0.0
    };

    let output = serde_json::json!({
        "total_buys": state.stats.total_buys,
        "total_sells": state.stats.total_sells,
        "successful_trades": state.stats.successful_trades,
        "failed_trades": state.stats.failed_trades,
        "total_invested_sol": state.stats.total_invested_sol,
        "total_returned_sol": state.stats.total_returned_sol,
        "total_pnl_sol": state.stats.total_returned_sol - state.stats.total_invested_sol,
        "session_pnl_sol": state.stats.session_pnl_sol,
        "win_count": wins,
        "loss_count": losses,
        "win_rate": format!("{:.1}%", win_rate),
        "signals_total": state.signals.len(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn cmd_history(limit: usize) -> Result<()> {
    let state = ScannerState::load()?;
    let trades: Vec<_> = state.trades.iter().rev().take(limit).collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "trades": trades,
            "total": state.trades.len(),
        }))?
    );
    Ok(())
}

async fn cmd_balance(config: &ScannerConfig) -> Result<()> {
    let client = ScannerClient::new()?;
    let balance = client.fetch_sol_balance().await?;
    let required = config.minimum_sol * 3.0 + engine::SOL_GAS_RESERVE;

    let output = serde_json::json!({
        "wallet": client.wallet,
        "balance_sol": balance,
        "suggested_minimum_sol": required,
        "sufficient": balance >= required,
        "hint": if balance >= required { "Ready to start" } else { "Please top up SOL" }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn cmd_tick(config: &ScannerConfig, dry_run: bool) -> Result<()> {
    let client = ScannerClient::new()?;
    let mut state = ScannerState::load()?;

    if state.stopped {
        bail!(
            "Bot stopped: {}. Run `strategy-memepump-scanner reset --force` to clear.",
            state.stop_reason.as_deref().unwrap_or("unknown")
        );
    }

    if let Some(reason) = state.check_circuit_breaker() {
        bail!("{}", reason);
    }

    if state.is_paused() {
        let output = serde_json::json!({
            "tick_time": chrono::Utc::now().to_rfc3339(),
            "actions": [{"type": "paused", "until": state.paused_until}],
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let now_ts = chrono::Utc::now().timestamp();
    let mut actions = Vec::new();

    // ── Exit checks for existing positions ──────────────────────────
    let ep = config.exit_params();
    let position_tokens: Vec<String> = state.positions.keys().cloned().collect();

    for token_addr in &position_tokens {
        let mut pos = match state.positions.get(token_addr) {
            Some(p) => p.clone(),
            None => continue,
        };

        let price = match client
            .get_price_info(token_addr)
            .await
            .ok()
            .and_then(|info| {
                let p = safe_float(&info["price"], 0.0);
                if p > 0.0 { Some(p) } else { None }
            }) {
            Some(p) => p,
            None => continue,
        };

        if pos.entry_price <= 0.0 {
            continue;
        }
        let pnl_pct = (price - pos.entry_price) / pos.entry_price * 100.0;
        let entry_ts = chrono::DateTime::parse_from_rfc3339(&pos.entry_time)
            .map(|t| t.timestamp())
            .unwrap_or(now_ts);
        let age_min = (now_ts - entry_ts).max(0) as f64 / 60.0;

        // Update peak
        if price > pos.peak_price {
            pos.peak_price = price;
        }

        let exit = check_exit(
            pnl_pct,
            age_min,
            pos.peak_price,
            price,
            pos.tp1_hit,
            pos.tier,
            pos.launch,
            pos.breakeven_pct,
            &ep,
        );

        if let Some(action) = exit {
            let sell_pct = exit_sell_pct(action);
            let (sell_raw, remaining_raw) = split_sell_raw(&pos.token_amount_raw, sell_pct);
            let is_full_exit = sell_pct >= 1.0 || remaining_raw == "0";

            if dry_run {
                actions.push(serde_json::json!({
                    "type": "exit", "mode": "DRY_RUN",
                    "symbol": pos.symbol, "reason": action.as_str(),
                    "pnl_pct": format!("{:.1}%", pnl_pct), "sell_pct": sell_pct
                }));
                if is_full_exit {
                    state.positions.remove(token_addr);
                } else {
                    pos.tp1_hit = true;
                    pos.token_amount_raw = remaining_raw;
                    state.positions.insert(token_addr.clone(), pos.clone());
                }
            } else {
                match client
                    .sell_token(token_addr, &sell_raw, config.slippage(pos.tier))
                    .await
                {
                    Ok(result) => {
                        let sol_out = result.amount_out / 1e9;
                        let sol_fraction = pos.entry_sol * sell_pct;
                        let pnl_sol = sol_out - sol_fraction;

                        state.stats.total_sells += 1;
                        state.stats.total_returned_sol += sol_out;
                        state.stats.session_pnl_sol += pnl_sol;

                        if pnl_sol < 0.0 {
                            state.record_loss(pnl_sol.abs());
                        } else {
                            state.record_win();
                        }

                        state.push_trade(Trade {
                            time: chrono::Utc::now().to_rfc3339(),
                            token_address: token_addr.clone(),
                            symbol: pos.symbol.clone(),
                            direction: "SELL".to_string(),
                            sol_amount: sol_out,
                            price,
                            tier: pos.tier,
                            launch: pos.launch,
                            tx_hash: result.tx_hash.clone(),
                            success: true,
                            exit_reason: Some(action.as_str().to_string()),
                            pnl_sol: Some(pnl_sol),
                        });

                        actions.push(serde_json::json!({
                            "type": "exit", "symbol": pos.symbol,
                            "reason": action.as_str(), "pnl_sol": pnl_sol,
                            "pnl_pct": format!("{:.1}%", pnl_pct), "sell_pct": sell_pct,
                            "tx_hash": result.tx_hash
                        }));

                        if is_full_exit {
                            state.positions.remove(token_addr);
                        } else {
                            // Partial exit: mark TP1 hit, update remaining raw
                            pos.tp1_hit = true;
                            pos.token_amount_raw = remaining_raw;
                            pos.entry_sol -= sol_fraction;
                            state.positions.insert(token_addr.clone(), pos.clone());
                        }

                        state.stats.successful_trades += 1;
                    }
                    Err(e) => {
                        pos.sell_fail_count += 1;
                        state.positions.insert(token_addr.clone(), pos.clone());
                        actions.push(serde_json::json!({
                            "type": "exit_failed", "symbol": pos.symbol, "error": e.to_string()
                        }));
                        state.errors.consecutive_errors += 1;
                        state.errors.last_error_time =
                            Some(chrono::Utc::now().to_rfc3339());
                        state.errors.last_error_msg = Some(e.to_string());
                    }
                }
            }
        } else {
            // Update position (peak price tracking)
            state.positions.insert(token_addr.clone(), pos);
        }
    }

    // ── Session risk check ───────────────────────────────────────────
    let now_str = chrono::Utc::now().to_rfc3339();
    if let Some(risk_reason) = check_session_risk(
        state.stats.consecutive_losses,
        state.stats.cumulative_loss_sol,
        state.paused_until.as_deref(),
        &now_str,
    ) {
        if risk_reason.contains("terminated") {
            state.stopped = true;
            state.stop_reason = Some(risk_reason.clone());
            state.save()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "tick_time": now_str,
                    "actions": [{"type": "session_stop", "reason": risk_reason}],
                }))?
            );
            return Ok(());
        } else if risk_reason.contains("Paused") {
            // Already paused; skip new buys
            state.save()?;
            let output = serde_json::json!({
                "tick_time": now_str,
                "actions": [{"type": "paused", "reason": risk_reason}],
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        } else {
            // Trigger pause
            let pause_until = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::seconds(engine::PAUSE_CONSEC_SEC as i64))
                .map(|t| t.to_rfc3339());
            state.paused_until = pause_until.clone();
            state.save()?;
            let output = serde_json::json!({
                "tick_time": now_str,
                "actions": [{"type": "pause_triggered", "reason": risk_reason, "until": pause_until}],
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }
    }

    // ── Scan for new buys ────────────────────────────────────────────
    let params = build_api_params(config);
    let tokens = match client.get_memepump_list(&params).await {
        Ok(t) => t,
        Err(e) => {
            state.errors.consecutive_errors += 1;
            state.errors.last_error_time = Some(chrono::Utc::now().to_rfc3339());
            state.errors.last_error_msg = Some(e.to_string());
            state.save()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "tick_time": now_str,
                    "actions": [{"type": "no_data", "error": e.to_string()}],
                }))?
            );
            return Ok(());
        }
    };

    for token_json in &tokens {
        let td = match parse_token_data(token_json) {
            Some(t) => t,
            None => continue,
        };
        let addr = td.token_address.clone();

        if state.positions.contains_key(&addr) {
            continue;
        }
        if state.prev_tx.contains(&addr) {
            continue;
        }
        state.prev_tx.insert(addr.clone());
        state.trim_prev_tx();

        if state.positions.len() >= config.max_positions {
            break;
        }

        // Layer 2: Client-side filter
        let _classify = match classify_token(&td) {
            Some(c) => c,
            None => {
                actions.push(serde_json::json!({
                    "type": "skip", "symbol": td.symbol,
                    "reason": format!("classify_failed (bs={:.2})", if td.sell_tx_1h > 0 { td.buy_tx_1h as f64 / td.sell_tx_1h as f64 } else { f64::MAX })
                }));
                continue;
            }
        };

        // Layer 3: Deep safety (fetch dev + bundle info)
        let dev_info = match client.get_dev_info(&addr).await {
            Ok(d) => d,
            Err(_) => serde_json::json!({}),
        };
        let bundle_info = match client.get_bundle_info(&addr).await {
            Ok(b) => b,
            Err(_) => serde_json::json!({}),
        };

        let dev_rug = safe_u32(&dev_info["rugPullCount"], 0);
        let dev_launched = safe_u32(&dev_info["tokenLaunchedCount"], safe_u32(&dev_info["totalLaunched"], 0));
        let dev_hold = safe_float(&dev_info["devHoldingPercent"], safe_float(&dev_info["devHoldPercent"], 0.0));
        let bundler_ath = safe_float(&bundle_info["bundlerAthPercent"], safe_float(&bundle_info["athPercent"], 0.0));
        let bundler_count = safe_u32(&bundle_info["bundlerCount"], safe_u32(&bundle_info["count"], 0));

        let verdict = deep_safety_check_with(
            dev_rug,
            dev_launched,
            dev_hold,
            bundler_ath,
            bundler_count,
            config.ds_max_dev_hold,
            config.ds_max_bundler_ath,
            config.ds_max_bundler_count,
        );

        if let engine::SafetyVerdict::Unsafe(reason) = verdict {
            actions.push(serde_json::json!({
                "type": "skip", "symbol": td.symbol, "reason": reason.as_str()
            }));
            state.push_signal(SignalRecord {
                time: chrono::Utc::now().to_rfc3339(),
                token_address: addr.clone(),
                symbol: td.symbol.clone(),
                tier: SignalTier::Scalp,
                launch: LaunchType::Quiet,
                sig_a_ratio: 0.0,
                sig_b_ratio: 0.0,
                market_cap: td.market_cap,
                acted: false,
                skip_reason: Some(reason.as_str().to_string()),
            });
            continue;
        }

        // Signal detection via candles
        let candles = client.get_candles(&addr, 7).await.unwrap_or(serde_json::json!([]));
        let (current_vol, prev_vols) = extract_candle_vols(&candles);
        let launch = classify_launch(current_vol);

        // Approximate signal_a using volume velocity vs prev average
        let prev_avg = if prev_vols.is_empty() {
            0.0
        } else {
            prev_vols.iter().sum::<f64>() / prev_vols.len() as f64
        };
        let (sig_a, sig_a_ratio) = if prev_avg > 0.0 {
            engine::check_signal_a(
                (current_vol / prev_avg * 10.0) as u32,
                60,
                10,
                launch,
            )
        } else {
            (false, 0.0)
        };

        let (sig_b, sig_b_ratio) = engine::check_signal_b(current_vol, &prev_vols, launch);
        let sig_c = engine::check_signal_c(td.buy_tx_1h, td.sell_tx_1h);

        let tier = match detect_signal(sig_a, sig_b, sig_c) {
            Some(t) => t,
            None => {
                actions.push(serde_json::json!({
                    "type": "skip", "symbol": td.symbol,
                    "reason": format!("no_signal (a={sig_a}, b={sig_b}, c={sig_c}, b_ratio={sig_b_ratio:.2})")
                }));
                state.push_signal(SignalRecord {
                    time: chrono::Utc::now().to_rfc3339(),
                    token_address: addr.clone(),
                    symbol: td.symbol.clone(),
                    tier: SignalTier::Scalp,
                    launch,
                    sig_a_ratio,
                    sig_b_ratio,
                    market_cap: td.market_cap,
                    acted: false,
                    skip_reason: Some("no_signal".to_string()),
                });
                continue;
            }
        };

        let sol_amount = config.position_size(tier);
        let slippage = config.slippage(tier);
        let breakeven_pct = calc_breakeven_pct(sol_amount);

        // Log signal
        state.push_signal(SignalRecord {
            time: chrono::Utc::now().to_rfc3339(),
            token_address: addr.clone(),
            symbol: td.symbol.clone(),
            tier,
            launch,
            sig_a_ratio,
            sig_b_ratio,
            market_cap: td.market_cap,
            acted: !dry_run,
            skip_reason: None,
        });

        if dry_run {
            actions.push(serde_json::json!({
                "type": "buy", "mode": "DRY_RUN",
                "symbol": td.symbol,
                "tier": format!("{:?}", tier),
                "launch": format!("{:?}", launch),
                "sol_amount": sol_amount,
                "sig_a_ratio": sig_a_ratio,
                "sig_b_ratio": sig_b_ratio,
                "market_cap": td.market_cap,
            }));
        } else {
            let price_info = client.get_price_info(&addr).await.unwrap_or(serde_json::json!({}));
            let price = safe_float(&price_info["price"], 0.0);

            match client.buy_token(&addr, sol_amount, slippage).await {
                Ok(result) => {
                    let amount_raw = format!("{}", result.amount_out as u64);
                    state.positions.insert(
                        addr.clone(),
                        Position {
                            token_address: addr.clone(),
                            symbol: td.symbol.clone(),
                            tier,
                            launch,
                            entry_price: price,
                            entry_sol: sol_amount,
                            token_amount_raw: amount_raw.clone(),
                            entry_time: chrono::Utc::now().to_rfc3339(),
                            peak_price: price,
                            tp1_hit: false,
                            breakeven_pct,
                            sell_fail_count: 0,
                        },
                    );

                    state.stats.total_buys += 1;
                    state.stats.total_invested_sol += sol_amount;

                    state.push_trade(Trade {
                        time: chrono::Utc::now().to_rfc3339(),
                        token_address: addr.clone(),
                        symbol: td.symbol.clone(),
                        direction: "BUY".to_string(),
                        sol_amount,
                        price,
                        tier,
                        launch,
                        tx_hash: result.tx_hash.clone(),
                        success: true,
                        exit_reason: None,
                        pnl_sol: None,
                    });

                    actions.push(serde_json::json!({
                        "type": "buy", "symbol": td.symbol,
                        "tier": format!("{:?}", tier), "launch": format!("{:?}", launch),
                        "sol_amount": sol_amount, "price": price,
                        "tx_hash": result.tx_hash, "amount_raw": amount_raw,
                    }));

                    state.errors.consecutive_errors = 0;
                }
                Err(e) => {
                    state.errors.consecutive_errors += 1;
                    state.errors.last_error_time = Some(chrono::Utc::now().to_rfc3339());
                    state.errors.last_error_msg = Some(e.to_string());
                    state.stats.failed_trades += 1;
                    actions.push(serde_json::json!({
                        "type": "buy_failed", "symbol": td.symbol, "error": e.to_string()
                    }));
                }
            }
        }
    }

    // Reset error counter on clean tick
    if !actions
        .iter()
        .any(|a| a["type"] == "buy_failed" || a["type"] == "exit_failed")
    {
        state.errors.consecutive_errors = 0;
    }

    state.dry_run = dry_run;
    state.save()?;

    let output = serde_json::json!({
        "tick_time": now_str,
        "positions": state.positions.len(),
        "session_pnl_sol": state.stats.session_pnl_sol,
        "actions": actions,
        "dry_run": dry_run,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn cmd_start(config: &ScannerConfig, dry_run: bool) -> Result<()> {
    let pid_path = ScannerState::pid_path();
    let dir = pid_path.parent().unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(dir)?;
    std::fs::write(&pid_path, format!("{}", std::process::id()))?;

    eprintln!(
        "Starting memepump scanner (tick every {}s)... Press Ctrl+C to stop.",
        config.tick_interval_secs
    );

    loop {
        if let Err(e) = cmd_tick(config, dry_run).await {
            eprintln!("Tick error: {}", e);
        }
        tokio::time::sleep(std::time::Duration::from_secs(config.tick_interval_secs)).await;
    }
}

fn cmd_stop() -> Result<()> {
    let pid_path = ScannerState::pid_path();
    if !pid_path.exists() {
        bail!("No running bot found (no PID file).");
    }
    let pid: i32 = std::fs::read_to_string(&pid_path)?.trim().parse()?;
    #[cfg(unix)]
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
    std::fs::remove_file(&pid_path)?;
    println!("{}", serde_json::json!({"stopped": true, "pid": pid}));
    Ok(())
}

async fn cmd_sell_all() -> Result<()> {
    let client = ScannerClient::new()?;
    let mut state = ScannerState::load()?;
    let mut results = Vec::new();

    let tokens: Vec<(String, String, String, SignalTier)> = state
        .positions
        .iter()
        .map(|(addr, pos)| {
            (
                addr.clone(),
                pos.symbol.clone(),
                pos.token_amount_raw.clone(),
                pos.tier,
            )
        })
        .collect();

    for (addr, symbol, amount_raw, tier) in &tokens {
        let slippage = match tier {
            SignalTier::Scalp => engine::SLIPPAGE_SCALP,
            SignalTier::Minimum => engine::SLIPPAGE_MINIMUM,
        };
        match client.sell_token(addr, amount_raw, slippage).await {
            Ok(result) => {
                state.positions.remove(addr);
                results.push(serde_json::json!({
                    "symbol": symbol, "status": "sold", "tx_hash": result.tx_hash
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "symbol": symbol, "status": "failed", "error": e.to_string()
                }));
            }
        }
    }

    state.save()?;
    let sold = results.iter().filter(|r| r["status"] == "sold").count();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "sold": sold,
            "failed": results.len() - sold,
            "results": results
        }))?
    );
    Ok(())
}

async fn cmd_sell(token_address: &str, amount_raw: &str) -> Result<()> {
    let client = ScannerClient::new()?;
    // Default to MINIMUM slippage for manual sells
    let result = client
        .sell_token(token_address, amount_raw, engine::SLIPPAGE_MINIMUM)
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "token": token_address,
            "tx_hash": result.tx_hash,
            "amount_out": result.amount_out,
        }))?
    );
    Ok(())
}

async fn cmd_test_trade(token_address: &str, amount_sol: f64) -> Result<()> {
    let client = ScannerClient::new()?;
    let price_info = client.get_price_info(token_address).await.unwrap_or(serde_json::json!({}));
    let price_before = safe_float(&price_info["price"], 0.0);

    eprintln!("Buying {amount_sol} SOL of {token_address}...");
    let buy = client
        .buy_token(token_address, amount_sol, engine::SLIPPAGE_MINIMUM)
        .await?;

    eprintln!("Waiting 3s...");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let amount_raw = format!("{}", buy.amount_out as u64);
    eprintln!("Selling...");
    let sell = client
        .sell_token(token_address, &amount_raw, engine::SLIPPAGE_MINIMUM)
        .await?;

    let price_info2 = client.get_price_info(token_address).await.unwrap_or(serde_json::json!({}));
    let price_after = safe_float(&price_info2["price"], 0.0);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "token": token_address, "amount_sol": amount_sol,
            "buy": {"tx_hash": buy.tx_hash, "price": price_before, "amount_out": buy.amount_out},
            "sell": {"tx_hash": sell.tx_hash, "amount_out": sell.amount_out},
            "price_before": price_before, "price_after": price_after,
        }))?
    );
    Ok(())
}

fn cmd_reset(force: bool) -> Result<()> {
    if !force {
        bail!("Reset requires --force flag.");
    }
    ScannerState::reset()?;
    println!("{}", serde_json::json!({"reset": true}));
    Ok(())
}
