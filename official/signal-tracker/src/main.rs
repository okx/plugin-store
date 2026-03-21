use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use signal_tracker::client::SignalClient;
use signal_tracker::config::SignalTrackerConfig;
use signal_tracker::engine::{
    self, calc_breakeven, calc_position_tier, check_exits, check_honeypot, check_k1_pump,
    check_session_risk, config_summary, run_dev_bundler_checks, run_safety_checks,
    run_signal_prefilter, safe_float, wallet_type_label, Position, Trade,
};
use signal_tracker::state::SignalTrackerState;

#[derive(Parser)]
#[command(
    name = "strategy-signal-tracker",
    version,
    about = "Signal Tracker — smart money signal-based Solana token trading"
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
    let config = SignalTrackerConfig::load()?;

    match cli.command {
        Commands::Config => {
            let mut summary = config_summary();
            // Overlay actual config values
            summary["config_path"] = serde_json::json!(
                SignalTrackerConfig::config_path().display().to_string()
            );
            println!("{}", serde_json::to_string_pretty(&summary)?);
            Ok(())
        }
        Commands::Status => cmd_status(),
        Commands::Report => cmd_report(),
        Commands::History { limit } => cmd_history(limit),
        Commands::Balance => cmd_balance(&config).await,
        Commands::Tick { dry_run } => cmd_tick(&config, dry_run).await,
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

/// Compute sell_amount_raw and remaining_raw from total and sell fraction.
fn split_sell_raw(amount_raw: &str, sell_pct: f64) -> (String, String) {
    let total: u64 = amount_raw.parse().unwrap_or(0);
    let sell = ((total as f64 * sell_pct).round() as u64).min(total);
    let remaining = total - sell;
    (sell.to_string(), remaining.to_string())
}

fn check_pid_file() -> bool {
    let pid_path = SignalTrackerState::pid_path();
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

fn cmd_status() -> Result<()> {
    let state = SignalTrackerState::load()?;
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
                "label": pos.label,
                "tier": pos.tier,
                "buy_price": pos.buy_price,
                "buy_amount_sol": pos.buy_amount_sol,
                "tp_tier": pos.tp_tier,
                "trailing_active": pos.trailing_active,
                "peak_pnl_pct": format!("{:.1}%", pos.peak_pnl_pct),
            })
        }).collect::<Vec<_>>(),
        "known_tokens": state.known_tokens.len(),
        "session_pnl_sol": state.stats.session_pnl_sol,
        "total_buys": state.stats.total_buys,
        "total_sells": state.stats.total_sells,
        "consecutive_losses": state.stats.consecutive_losses,
        "cumulative_loss_sol": state.stats.cumulative_loss_sol,
        "paused_until": state.paused_until,
        "consecutive_errors": state.errors.consecutive_errors,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn cmd_report() -> Result<()> {
    let state = SignalTrackerState::load()?;

    let sell_trades: Vec<_> = state
        .trades
        .iter()
        .filter(|t| t.action == "SELL")
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
        "positions": state.positions.len(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn cmd_history(limit: usize) -> Result<()> {
    let state = SignalTrackerState::load()?;
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

async fn cmd_balance(config: &SignalTrackerConfig) -> Result<()> {
    let client = SignalClient::new()?;
    let balance = client.fetch_sol_balance().await?;
    let required = config.position_high_sol * config.max_positions as f64 + config.gas_reserve_sol;

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

async fn cmd_tick(config: &SignalTrackerConfig, dry_run: bool) -> Result<()> {
    let client = SignalClient::new()?;
    let mut state = SignalTrackerState::load()?;

    if state.stopped {
        bail!(
            "Bot stopped: {}. Run `strategy-signal-tracker reset --force` to clear.",
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
    let position_tokens: Vec<String> = state.positions.keys().cloned().collect();

    for token_addr in &position_tokens {
        let mut pos = match state.positions.get(token_addr) {
            Some(p) => p.clone(),
            None => continue,
        };

        let price_info = match client.fetch_price_info(token_addr).await {
            Ok(info) => info,
            Err(_) => continue,
        };

        let price = safe_float(&price_info["price"], 0.0);
        let liq = safe_float(&price_info["liquidity"], 0.0);
        let mc = safe_float(&price_info["marketCap"], 0.0);

        if price <= 0.0 {
            continue;
        }

        let exit_signal = check_exits(&mut pos, price, liq, mc, now_ts);

        if let Some(signal) = exit_signal {
            let amount_raw = get_position_amount_raw(&state, token_addr);
            let (sell_raw, remaining_raw) = split_sell_raw(&amount_raw, signal.sell_pct);
            let is_full_exit = signal.sell_pct >= 1.0
                || remaining_raw == "0"
                || remaining_raw.parse::<u64>().unwrap_or(0) == 0;

            if dry_run {
                actions.push(serde_json::json!({
                    "type": "exit", "mode": "DRY_RUN",
                    "symbol": pos.symbol, "reason": signal.reason,
                    "sell_pct": signal.sell_pct,
                }));
                if is_full_exit {
                    state.positions.remove(token_addr);
                } else {
                    update_position_amount(&mut state, token_addr, &remaining_raw, &mut pos);
                }
            } else {
                match client.sell_token(token_addr, &sell_raw).await {
                    Ok(result) => {
                        let sol_out = result.amount_out / 1e9;
                        let sol_fraction = pos.buy_amount_sol * signal.sell_pct;
                        let pnl_sol = sol_out - sol_fraction;
                        let pnl_pct = (price - pos.buy_price) / pos.buy_price * 100.0;
                        let net_pnl_pct = pnl_pct - pos.breakeven_pct;

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
                            symbol: pos.symbol.clone(),
                            token_address: token_addr.clone(),
                            label: pos.label.clone(),
                            tier: pos.tier.clone(),
                            action: "SELL".to_string(),
                            price,
                            amount_sol: sol_out,
                            entry_mc: Some(pos.entry_mc),
                            exit_mc: Some(mc),
                            exit_reason: Some(signal.reason.clone()),
                            pnl_pct: Some(pnl_pct),
                            net_pnl_pct: Some(net_pnl_pct),
                            pnl_sol: Some(pnl_sol),
                            tx_hash: result.tx_hash.clone().unwrap_or_default(),
                        });

                        actions.push(serde_json::json!({
                            "type": "exit", "symbol": pos.symbol,
                            "reason": signal.reason,
                            "pnl_sol": pnl_sol,
                            "pnl_pct": format!("{:.1}%", pnl_pct),
                            "net_pnl_pct": format!("{:.1}%", net_pnl_pct),
                            "sell_pct": signal.sell_pct,
                            "tx_hash": result.tx_hash,
                        }));

                        if is_full_exit {
                            state.positions.remove(token_addr);
                        } else {
                            // Partial: update remaining amount and buy_amount_sol
                            pos.buy_amount_sol -= sol_fraction;
                            update_position_amount(&mut state, token_addr, &remaining_raw, &mut pos);
                        }

                        state.stats.successful_trades += 1;
                        state.errors.consecutive_errors = 0;
                    }
                    Err(e) => {
                        state.errors.consecutive_errors += 1;
                        state.errors.last_error_time =
                            Some(chrono::Utc::now().to_rfc3339());
                        state.errors.last_error_msg = Some(e.to_string());
                        state.stats.failed_trades += 1;
                        actions.push(serde_json::json!({
                            "type": "exit_failed", "symbol": pos.symbol, "error": e.to_string()
                        }));
                        // Keep position in state with updated peak tracking
                        state.positions.insert(token_addr.clone(), pos);
                    }
                }
            }
        } else {
            // Keep position with updated peak info from check_exits
            state.positions.insert(token_addr.clone(), pos);
        }
    }

    // ── Session risk check ───────────────────────────────────────────
    let now_ts_for_risk = chrono::Utc::now().timestamp();
    if let Some((risk_reason, pause_secs)) = check_session_risk(
        state.stats.consecutive_losses,
        state.stats.cumulative_loss_sol,
    ) {
        if pause_secs == u64::MAX {
            state.stopped = true;
            state.stop_reason = Some(risk_reason.clone());
            state.save()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "tick_time": chrono::Utc::now().to_rfc3339(),
                    "actions": [{"type": "session_stop", "reason": risk_reason}],
                }))?
            );
            return Ok(());
        } else {
            let until_ts = now_ts_for_risk + pause_secs as i64;
            state.paused_until = Some(until_ts);
            state.save()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "tick_time": chrono::Utc::now().to_rfc3339(),
                    "actions": [{"type": "pause_triggered", "reason": risk_reason, "until_ts": until_ts}],
                }))?
            );
            return Ok(());
        }
    }

    // ── Check position limit ─────────────────────────────────────────
    if state.positions.len() >= config.max_positions {
        state.save()?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "tick_time": chrono::Utc::now().to_rfc3339(),
                "positions": state.positions.len(),
                "actions": [{"type": "max_positions_reached"}],
            }))?
        );
        return Ok(());
    }

    // ── Fetch signals ────────────────────────────────────────────────
    let signals = match client.fetch_signals().await {
        Ok(s) => s,
        Err(e) => {
            state.errors.consecutive_errors += 1;
            state.errors.last_error_time = Some(chrono::Utc::now().to_rfc3339());
            state.errors.last_error_msg = Some(e.to_string());
            state.save()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "tick_time": chrono::Utc::now().to_rfc3339(),
                    "actions": [{"type": "no_signals", "error": e.to_string()}],
                }))?
            );
            return Ok(());
        }
    };

    for signal in &signals {
        let token_addr = match signal["token"]["tokenContractAddress"]
            .as_str()
            .or_else(|| signal["tokenContractAddress"].as_str())
        {
            Some(a) => a.to_string(),
            None => continue,
        };

        let symbol = signal["token"]["tokenSymbol"]
            .as_str()
            .or_else(|| signal["tokenSymbol"].as_str())
            .unwrap_or("?")
            .to_string();

        if state.known_tokens.contains(&token_addr) {
            continue;
        }
        if state.positions.contains_key(&token_addr) {
            continue;
        }
        state.known_tokens.insert(token_addr.clone());
        state.trim_known_tokens();

        if state.positions.len() >= config.max_positions {
            break;
        }

        // Layer 1: Signal pre-filter
        let (passed, reasons) = run_signal_prefilter(signal);
        if !passed {
            actions.push(serde_json::json!({
                "type": "skip", "symbol": symbol, "reason": reasons.join("; ")
            }));
            continue;
        }

        let wallet_count = engine::safe_int(&signal["triggerWalletCount"], 0) as u32;
        let wallet_type = signal["walletType"]
            .as_str()
            .or_else(|| signal["labelType"].as_str())
            .unwrap_or("1");
        let label = wallet_type_label(wallet_type).to_string();

        // Layer 2: Safety checks (fetch fresh price info)
        let price_info = match client.fetch_price_info(&token_addr).await {
            Ok(info) => info,
            Err(e) => {
                actions.push(serde_json::json!({
                    "type": "skip", "symbol": symbol,
                    "reason": format!("price-info: {}", e)
                }));
                continue;
            }
        };

        let (passed, reasons) = run_safety_checks(&price_info);
        if !passed {
            actions.push(serde_json::json!({
                "type": "skip", "symbol": symbol, "reason": reasons.join("; ")
            }));
            continue;
        }

        // Layer 3: Dev/Bundler checks
        let dev_info = client
            .fetch_dev_info(&token_addr)
            .await
            .unwrap_or(serde_json::json!({}));
        let bundle_info = client
            .fetch_bundle_info(&token_addr)
            .await
            .unwrap_or(serde_json::json!({}));

        let (passed, reasons) = run_dev_bundler_checks(&dev_info, &bundle_info);
        if !passed {
            actions.push(serde_json::json!({
                "type": "skip", "symbol": symbol, "reason": reasons.join("; ")
            }));
            continue;
        }

        // Layer 4: k1 pump check
        let candles = client
            .fetch_candles_1m(&token_addr)
            .await
            .unwrap_or(serde_json::json!([]));
        if let Some(reason) = check_k1_pump(&candles) {
            actions.push(serde_json::json!({
                "type": "skip", "symbol": symbol, "reason": reason
            }));
            continue;
        }

        // Layer 5: Honeypot check
        let quote = client
            .fetch_quote(&token_addr, 0.01)
            .await
            .unwrap_or(serde_json::json!({}));
        if let Some(reason) = check_honeypot(&quote) {
            actions.push(serde_json::json!({
                "type": "skip", "symbol": symbol, "reason": reason
            }));
            continue;
        }

        // Position sizing
        let (tier, sol_amount) = calc_position_tier(wallet_count);
        let breakeven_pct = calc_breakeven(sol_amount);
        let price = safe_float(&price_info["price"], 0.0);
        let mc = safe_float(&price_info["marketCap"], 0.0);

        if dry_run {
            actions.push(serde_json::json!({
                "type": "buy", "mode": "DRY_RUN",
                "symbol": symbol, "label": label, "tier": tier,
                "sol_amount": sol_amount, "wallet_count": wallet_count,
                "mc": format!("${mc:.0}"),
            }));
        } else {
            match client.buy_token(&token_addr, sol_amount).await {
                Ok(result) => {
                    let amount_raw = format!("{}", result.amount_out as u64);

                    let mut pos = Position {
                        token_address: token_addr.clone(),
                        symbol: symbol.clone(),
                        label: label.clone(),
                        tier: tier.to_string(),
                        buy_price: price,
                        buy_amount_sol: sol_amount,
                        buy_time: chrono::Utc::now().to_rfc3339(),
                        breakeven_pct,
                        peak_price: price,
                        peak_pnl_pct: 0.0,
                        trailing_active: false,
                        tp_tier: 0,
                        entry_mc: mc,
                        tx_hash: result.tx_hash.clone().unwrap_or_default(),
                    };

                    // Store amount_raw in tx_hash field as a workaround
                    // (Position struct uses tx_hash for the buy tx; we track amount separately)
                    // We store amount_raw in the trades record and retrieve it there.
                    state.positions.insert(token_addr.clone(), pos.clone());

                    // Store amount_raw as a separate key in known_tokens
                    // Actually, we embed it in the position: repurpose tx_hash after storing.
                    // Better: add it to trade record and retrieve it per-trade.
                    // For clean tracking, insert a second entry in a state map.
                    // We'll use the Trade history to reconstruct amount_raw.
                    // For now, store in tx_hash temporarily.
                    pos.tx_hash = amount_raw.clone();
                    state.positions.insert(token_addr.clone(), pos);

                    state.stats.total_buys += 1;
                    state.stats.total_invested_sol += sol_amount;

                    state.push_trade(Trade {
                        time: chrono::Utc::now().to_rfc3339(),
                        symbol: symbol.clone(),
                        token_address: token_addr.clone(),
                        label: label.clone(),
                        tier: tier.to_string(),
                        action: "BUY".to_string(),
                        price,
                        amount_sol: sol_amount,
                        entry_mc: Some(mc),
                        exit_mc: None,
                        exit_reason: None,
                        pnl_pct: None,
                        net_pnl_pct: None,
                        pnl_sol: None,
                        tx_hash: amount_raw.clone(),
                    });

                    actions.push(serde_json::json!({
                        "type": "buy", "symbol": symbol,
                        "label": label, "tier": tier,
                        "sol_amount": sol_amount, "price": price,
                        "wallet_count": wallet_count,
                        "amount_raw": amount_raw,
                    }));

                    state.errors.consecutive_errors = 0;
                    state.stats.successful_trades += 1;
                }
                Err(e) => {
                    state.errors.consecutive_errors += 1;
                    state.errors.last_error_time =
                        Some(chrono::Utc::now().to_rfc3339());
                    state.errors.last_error_msg = Some(e.to_string());
                    state.stats.failed_trades += 1;
                    actions.push(serde_json::json!({
                        "type": "buy_failed", "symbol": symbol, "error": e.to_string()
                    }));
                }
            }
        }
    }

    state.dry_run = dry_run;
    state.save()?;

    let output = serde_json::json!({
        "tick_time": chrono::Utc::now().to_rfc3339(),
        "positions": state.positions.len(),
        "session_pnl_sol": state.stats.session_pnl_sol,
        "actions": actions,
        "dry_run": dry_run,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Get the stored token amount_raw for a position.
/// We store it in pos.tx_hash field (repurposed) when buying.
fn get_position_amount_raw(state: &SignalTrackerState, token_addr: &str) -> String {
    state
        .positions
        .get(token_addr)
        .map(|p| p.tx_hash.clone())
        .unwrap_or_default()
}

/// Update position with new remaining amount.
fn update_position_amount(
    state: &mut SignalTrackerState,
    token_addr: &str,
    remaining_raw: &str,
    pos: &mut Position,
) {
    pos.tx_hash = remaining_raw.to_string();
    state.positions.insert(token_addr.to_string(), pos.clone());
}

async fn cmd_start(config: &SignalTrackerConfig, dry_run: bool) -> Result<()> {
    let pid_path = SignalTrackerState::pid_path();
    let dir = pid_path.parent().unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(dir)?;
    std::fs::write(&pid_path, format!("{}", std::process::id()))?;

    eprintln!(
        "Starting signal tracker (tick every {}s)... Press Ctrl+C to stop.",
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
    let pid_path = SignalTrackerState::pid_path();
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
    let client = SignalClient::new()?;
    let mut state = SignalTrackerState::load()?;
    let mut results = Vec::new();

    let tokens: Vec<(String, String, String)> = state
        .positions
        .iter()
        .map(|(addr, pos)| (addr.clone(), pos.symbol.clone(), pos.tx_hash.clone()))
        .collect();

    for (addr, symbol, amount_raw) in &tokens {
        match client.sell_token(addr, amount_raw).await {
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
    let client = SignalClient::new()?;
    let result = client.sell_token(token_address, amount_raw).await?;
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
    let client = SignalClient::new()?;
    let price_before = client.fetch_price(token_address).await.unwrap_or(0.0);

    eprintln!("Buying {amount_sol} SOL of {token_address}...");
    let buy = client.buy_token(token_address, amount_sol).await?;

    eprintln!("Waiting 3s...");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let amount_raw = format!("{}", buy.amount_out as u64);
    eprintln!("Selling...");
    let sell = client.sell_token(token_address, &amount_raw).await?;

    let price_after = client.fetch_price(token_address).await.unwrap_or(0.0);

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
    SignalTrackerState::reset()?;
    println!("{}", serde_json::json!({"reset": true}));
    Ok(())
}
