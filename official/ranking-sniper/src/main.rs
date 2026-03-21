use std::collections::HashSet;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use ranking_sniper::client::SniperClient;
use ranking_sniper::config::SniperConfig;
use ranking_sniper::engine::{self, safe_float, Position, Trade};
use ranking_sniper::state::SniperState;

#[derive(Parser)]
#[command(name = "strategy-ranking-sniper", version, about = "SOL Ranking Sniper — automated Solana token sniping")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute one tick cycle
    Tick {
        #[arg(long)]
        budget: Option<f64>,
        #[arg(long)]
        per_trade: Option<f64>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Start continuous bot (tick every 10s)
    Start {
        #[arg(long)]
        budget: Option<f64>,
        #[arg(long)]
        per_trade: Option<f64>,
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
    /// Market analysis (current ranking)
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
    let mut config = SniperConfig::load()?;

    match cli.command {
        Commands::Analyze => cmd_analyze(&config).await,
        Commands::Config => { config.print_summary(); Ok(()) }
        Commands::Status => cmd_status(),
        Commands::Report => cmd_report(),
        Commands::History { limit } => cmd_history(limit),
        Commands::Balance => cmd_balance(&config).await,
        Commands::Tick { budget, per_trade, dry_run } => {
            apply_overrides(&mut config, budget, per_trade, dry_run);
            cmd_tick(&config).await
        }
        Commands::Start { budget, per_trade, dry_run } => {
            apply_overrides(&mut config, budget, per_trade, dry_run);
            cmd_start(&config).await
        }
        Commands::Stop => cmd_stop(),
        Commands::SellAll => cmd_sell_all().await,
        Commands::Sell { token_address, amount } => cmd_sell(&token_address, &amount).await,
        Commands::TestTrade { token_address, amount } => cmd_test_trade(&token_address, amount).await,
        Commands::Reset { force } => cmd_reset(force),
    }
}

fn apply_overrides(config: &mut SniperConfig, budget: Option<f64>, per_trade: Option<f64>, dry_run: bool) {
    if let Some(b) = budget { config.budget_sol = b; }
    if let Some(p) = per_trade { config.per_trade_sol = p; }
    if dry_run { config.dry_run = true; }
}

// ── Commands ──────────────────────────────────────────────────────

async fn cmd_analyze(config: &SniperConfig) -> Result<()> {
    let client = SniperClient::new_read_only()?;
    let ranking = client.fetch_ranking(config.top_n).await?;

    let output = serde_json::json!({
        "ranking_count": ranking.len(),
        "top_tokens": ranking.iter().take(10).map(|t| {
            serde_json::json!({
                "symbol": t["tokenSymbol"].as_str().unwrap_or("?"),
                "address": t["tokenContractAddress"].as_str().unwrap_or(""),
                "change_5m": format!("{:.1}%", safe_float(&t["priceChangePercent5M"], 0.0)),
                "change_24h": format!("{:.1}%", safe_float(&t["priceChangePercent24H"], 0.0)),
                "market_cap": format!("${:.0}", safe_float(&t["marketCap"], 0.0)),
                "liquidity": format!("${:.0}", safe_float(&t["liquidity"], 0.0)),
                "holders": safe_float(&t["holdersCount"], 0.0) as i64,
            })
        }).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn cmd_status() -> Result<()> {
    let state = SniperState::load()?;
    let pid_running = check_pid_file();

    let output = serde_json::json!({
        "bot_running": pid_running,
        "stopped": state.stopped,
        "stop_reason": state.stop_reason,
        "position_count": state.positions.len(),
        "positions": state.positions.iter().map(|(addr, pos)| {
            serde_json::json!({
                "token": addr,
                "symbol": pos.symbol,
                "buy_price": pos.buy_price,
                "buy_amount_sol": pos.buy_amount_sol,
                "score": pos.tp_sold.len(), // proxy
            })
        }).collect::<Vec<_>>(),
        "remaining_budget_sol": state.remaining_budget_sol,
        "daily_pnl_sol": state.stats.daily_pnl_sol,
        "known_tokens_count": state.known_tokens.len(),
        "consecutive_errors": state.errors.consecutive_errors,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn cmd_report() -> Result<()> {
    let state = SniperState::load()?;

    let sell_trades: Vec<_> = state.trades.iter().filter(|t| t.action == "SELL").collect();
    let wins = sell_trades.iter().filter(|t| t.pnl_sol.unwrap_or(0.0) > 0.0).count();
    let losses = sell_trades.len() - wins;
    let win_rate = if !sell_trades.is_empty() { wins as f64 / sell_trades.len() as f64 * 100.0 } else { 0.0 };

    let output = serde_json::json!({
        "total_buys": state.stats.total_buys,
        "total_sells": state.stats.total_sells,
        "total_invested_sol": state.stats.total_invested_sol,
        "total_returned_sol": state.stats.total_returned_sol,
        "total_pnl_sol": state.stats.total_returned_sol - state.stats.total_invested_sol,
        "daily_pnl_sol": state.stats.daily_pnl_sol,
        "win_count": wins,
        "loss_count": losses,
        "win_rate": format!("{:.1}%", win_rate),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn cmd_history(limit: usize) -> Result<()> {
    let state = SniperState::load()?;
    let trades: Vec<_> = state.trades.iter().rev().take(limit).collect();
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"trades": trades, "total": state.trades.len()}))?);
    Ok(())
}

async fn cmd_balance(config: &SniperConfig) -> Result<()> {
    let client = SniperClient::new()?;
    let balance = client.fetch_sol_balance().await?;
    let required = config.budget_sol + config.gas_reserve_sol;

    let output = serde_json::json!({
        "wallet": client.wallet,
        "balance_sol": balance,
        "required_sol": required,
        "sufficient": balance >= required,
        "hint": if balance >= required { "Ready to start" } else { "Please top up SOL to your wallet" }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn cmd_tick(config: &SniperConfig) -> Result<()> {
    let client = SniperClient::new()?;
    let mut state = SniperState::load()?;
    state.maybe_reset_daily();

    if state.stopped {
        bail!("Bot stopped: {}. Run `strategy-ranking-sniper reset --force` to clear.",
              state.stop_reason.as_deref().unwrap_or("unknown"));
    }

    if let Some(reason) = state.check_circuit_breaker(config) {
        bail!("{}", reason);
    }

    let ranking = match client.fetch_ranking(config.top_n).await {
        Ok(r) => r,
        Err(e) => {
            state.errors.consecutive_errors += 1;
            state.errors.last_error_time = Some(chrono::Utc::now().to_rfc3339());
            state.errors.last_error_msg = Some(e.to_string());
            state.save()?;
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({"actions": [{"type": "no_ranking_data", "error": e.to_string()}]}))?);
            return Ok(());
        }
    };

    // Build ranking set for exit checks
    let ranking_set: HashSet<String> = ranking.iter()
        .filter_map(|t| t["tokenContractAddress"].as_str().map(|s| s.to_string()))
        .collect();
    let now_ts = chrono::Utc::now().timestamp();

    let mut actions = Vec::new();

    // Check exits for existing positions
    let position_tokens: Vec<String> = state.positions.keys().cloned().collect();
    for token_addr in &position_tokens {
        let mut pos = match state.positions.get(token_addr) { Some(p) => p.clone(), None => continue };
        let price = match client.fetch_price(token_addr).await { Ok(p) => p, Err(_) => continue };

        let exit = engine::check_exits(&mut pos, price, &ranking_set, now_ts, config);
        if let Some(signal) = exit {
            if config.dry_run {
                actions.push(serde_json::json!({"type": "exit", "mode": "DRY_RUN", "symbol": pos.symbol, "reason": signal.reason}));
            } else {
                match client.sell_token(token_addr, &pos.amount_raw).await {
                    Ok(result) => {
                        let sol_out = result.amount_out / 1e9;
                        let pnl_sol = sol_out - pos.buy_amount_sol;
                        state.stats.total_sells += 1;
                        state.stats.total_returned_sol += sol_out;
                        state.stats.daily_pnl_sol += pnl_sol;
                        state.positions.remove(token_addr);
                        state.remaining_budget_sol += sol_out;
                        state.record_sell_time(token_addr);
                        state.push_trade(Trade {
                            time: chrono::Utc::now().to_rfc3339(),
                            symbol: pos.symbol.clone(),
                            token_address: token_addr.clone(),
                            action: "SELL".to_string(),
                            price,
                            amount_sol: sol_out,
                            score: None,
                            exit_reason: Some(signal.reason.clone()),
                            pnl_pct: Some((price - pos.buy_price) / pos.buy_price * 100.0),
                            pnl_sol: Some(pnl_sol),
                            tx_hash: result.tx_hash.unwrap_or_default(),
                        });
                        actions.push(serde_json::json!({"type": "exit", "symbol": pos.symbol, "reason": signal.reason, "pnl_sol": pnl_sol}));
                    }
                    Err(e) => {
                        actions.push(serde_json::json!({"type": "exit_failed", "symbol": pos.symbol, "error": e.to_string()}));
                    }
                }
            }
        } else {
            // Update position with peak tracking
            state.positions.insert(token_addr.clone(), pos);
        }
    }

    // Check daily loss
    let daily_loss_exceeded = engine::check_daily_loss(
        state.stats.daily_pnl_sol, config.budget_sol, config.daily_loss_limit_pct,
    ).is_some();

    // Scan for new entries
    for token in &ranking {
        let addr = match token["tokenContractAddress"].as_str() { Some(a) => a, None => continue };

        if state.known_tokens.contains(addr) || state.positions.contains_key(addr) { continue; }
        state.known_tokens.insert(addr.to_string());

        if state.positions.len() >= config.max_positions { continue; }
        if state.remaining_budget_sol < config.per_trade_sol { continue; }
        if state.is_cooldown_active(addr, config) { continue; }

        let symbol = token["tokenSymbol"].as_str().unwrap_or("?").to_string();

        // Layer 1: Slot Guard
        let (passed, reasons) = engine::run_slot_guard(
            token, state.positions.len(), state.positions.contains_key(addr),
            daily_loss_exceeded, state.is_cooldown_active(addr, config), config,
        );
        if !passed {
            actions.push(serde_json::json!({"type": "skip", "symbol": symbol, "reason": reasons.join("; ")}));
            continue;
        }

        // Layer 2: Advanced Safety
        let adv_info = match client.fetch_advanced_info(addr).await {
            Ok(info) => info,
            Err(e) => {
                actions.push(serde_json::json!({"type": "skip", "symbol": symbol, "reason": format!("advanced-info: {}", e)}));
                continue;
            }
        };
        let (passed, reasons) = engine::run_advanced_safety(&adv_info, config);
        if !passed {
            actions.push(serde_json::json!({"type": "skip", "symbol": symbol, "reason": reasons.join("; ")}));
            continue;
        }

        // Layer 3: Holder Risk
        let suspicious = client.fetch_holder_risk(addr, "6").await.unwrap_or_default();
        let phishing = client.fetch_holder_risk(addr, "8").await.unwrap_or_default();
        let suspicious_val = serde_json::Value::Array(suspicious.clone());
        let phishing_val = serde_json::Value::Array(phishing);
        let (passed, reasons) = engine::run_holder_risk_scan(&suspicious_val, &phishing_val, config);
        if !passed {
            actions.push(serde_json::json!({"type": "skip", "symbol": symbol, "reason": reasons.join("; ")}));
            continue;
        }

        // Momentum Score
        let suspicious_active = suspicious.iter().filter(|h| safe_float(&h["holdPercent"], 0.0) > 0.0).count();
        let score = engine::calc_momentum_score(token, &adv_info, suspicious_active);
        if score < config.score_buy_threshold {
            actions.push(serde_json::json!({"type": "skip", "symbol": symbol, "reason": format!("score {} < {}", score, config.score_buy_threshold)}));
            continue;
        }

        // Buy
        if config.dry_run {
            actions.push(serde_json::json!({"type": "buy", "mode": "DRY_RUN", "symbol": symbol, "score": score}));
        } else {
            let price = match client.fetch_price(addr).await { Ok(p) => p, Err(_) => continue };
            match client.buy_token(addr, config.per_trade_sol).await {
                Ok(result) => {
                    let amount_raw = format!("{}", result.amount_out as u64);
                    state.positions.insert(addr.to_string(), Position {
                        token_address: addr.to_string(),
                        symbol: symbol.clone(),
                        buy_price: price,
                        buy_amount_sol: config.per_trade_sol,
                        buy_time: chrono::Utc::now().to_rfc3339(),
                        peak_pnl_pct: 0.0,
                        trailing_active: false,
                        tp_sold: vec![],
                        tx_hash: result.tx_hash.clone().unwrap_or_default(),
                        amount_raw,
                    });
                    state.remaining_budget_sol -= config.per_trade_sol;
                    state.stats.total_buys += 1;
                    state.stats.total_invested_sol += config.per_trade_sol;
                    state.push_trade(Trade {
                        time: chrono::Utc::now().to_rfc3339(),
                        symbol: symbol.clone(),
                        token_address: addr.to_string(),
                        action: "BUY".to_string(),
                        price,
                        amount_sol: config.per_trade_sol,
                        score: Some(score),
                        exit_reason: None,
                        pnl_pct: None,
                        pnl_sol: None,
                        tx_hash: result.tx_hash.unwrap_or_default(),
                    });
                    actions.push(serde_json::json!({"type": "buy", "symbol": symbol, "score": score, "price": price}));
                }
                Err(e) => {
                    state.errors.consecutive_errors += 1;
                    actions.push(serde_json::json!({"type": "buy_failed", "symbol": symbol, "error": e.to_string()}));
                }
            }
        }
    }

    // Reset error counter on successful tick
    if !actions.iter().any(|a| a["type"] == "buy_failed" || a["type"] == "exit_failed") {
        state.errors.consecutive_errors = 0;
    }

    state.save()?;

    let output = serde_json::json!({
        "tick_time": chrono::Utc::now().to_rfc3339(),
        "positions": state.positions.len(),
        "remaining_budget_sol": state.remaining_budget_sol,
        "daily_pnl_sol": state.stats.daily_pnl_sol,
        "actions": actions,
        "dry_run": config.dry_run,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn cmd_start(config: &SniperConfig) -> Result<()> {
    config.print_summary();
    let pid_path = SniperState::pid_path();
    std::fs::create_dir_all(pid_path.parent().unwrap())?;
    std::fs::write(&pid_path, format!("{}", std::process::id()))?;

    eprintln!("Starting bot (tick every {}s)... Press Ctrl+C to stop.", config.tick_interval_secs);

    loop {
        if let Err(e) = cmd_tick(config).await {
            eprintln!("Tick error: {}", e);
        }
        tokio::time::sleep(std::time::Duration::from_secs(config.tick_interval_secs)).await;
    }
}

fn cmd_stop() -> Result<()> {
    let pid_path = SniperState::pid_path();
    if !pid_path.exists() {
        bail!("No running bot found (no PID file).");
    }
    let pid: i32 = std::fs::read_to_string(&pid_path)?.trim().parse()?;
    #[cfg(unix)]
    unsafe { libc::kill(pid, libc::SIGTERM); }
    std::fs::remove_file(&pid_path)?;
    println!("{}", serde_json::json!({"stopped": true, "pid": pid}));
    Ok(())
}

async fn cmd_sell_all() -> Result<()> {
    let client = SniperClient::new()?;
    let mut state = SniperState::load()?;
    let mut results = Vec::new();

    let tokens: Vec<(String, String, String)> = state.positions.iter()
        .map(|(addr, pos)| (addr.clone(), pos.symbol.clone(), pos.amount_raw.clone()))
        .collect();

    for (addr, symbol, amount_raw) in &tokens {
        match client.sell_token(addr, amount_raw).await {
            Ok(result) => {
                state.positions.remove(addr);
                state.record_sell_time(addr);
                results.push(serde_json::json!({"symbol": symbol, "status": "sold", "tx_hash": result.tx_hash}));
            }
            Err(e) => {
                results.push(serde_json::json!({"symbol": symbol, "status": "failed", "error": e.to_string()}));
            }
        }
    }

    state.save()?;
    let sold = results.iter().filter(|r| r["status"] == "sold").count();
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"sold": sold, "failed": results.len() - sold, "results": results}))?);
    Ok(())
}

async fn cmd_sell(token_address: &str, amount_raw: &str) -> Result<()> {
    let client = SniperClient::new()?;
    let result = client.sell_token(token_address, amount_raw).await?;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"token": token_address, "tx_hash": result.tx_hash, "amount_out": result.amount_out}))?);
    Ok(())
}

async fn cmd_test_trade(token_address: &str, amount_sol: f64) -> Result<()> {
    let client = SniperClient::new()?;
    let price_before = client.fetch_price(token_address).await?;
    eprintln!("Buying {} SOL of {}...", amount_sol, token_address);
    let buy = client.buy_token(token_address, amount_sol).await?;
    eprintln!("Waiting 3s...");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let amount_raw = format!("{}", buy.amount_out as u64);
    eprintln!("Selling...");
    let sell = client.sell_token(token_address, &amount_raw).await?;
    let price_after = client.fetch_price(token_address).await.unwrap_or(0.0);
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "token": token_address, "amount_sol": amount_sol,
        "buy": {"tx_hash": buy.tx_hash, "price": price_before, "amount_out": buy.amount_out},
        "sell": {"tx_hash": sell.tx_hash, "amount_out": sell.amount_out},
        "price_before": price_before, "price_after": price_after,
    }))?);
    Ok(())
}

fn cmd_reset(force: bool) -> Result<()> {
    if !force { bail!("Reset requires --force flag."); }
    SniperState::reset()?;
    println!("{}", serde_json::json!({"reset": true}));
    Ok(())
}

fn check_pid_file() -> bool {
    let pid_path = SniperState::pid_path();
    if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            #[cfg(unix)]
            unsafe { return libc::kill(pid, 0) == 0; }
        }
    }
    false
}
