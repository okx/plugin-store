mod api;
mod auth;
mod commands;
mod config;
mod onchainos;
mod sanitize;
mod series;
mod signing;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "polymarket",
    version,
    about = "Trade prediction markets on Polymarket — buy and sell YES/NO outcome tokens on Polygon (chain 137)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check wallet assets and get a recommended next step (region check, balances, positions, onboarding guidance)
    Quickstart(commands::quickstart::QuickstartArgs),

    /// Check whether Polymarket is accessible from your current IP (run before topping up USDC)
    CheckAccess,

    /// List active prediction markets (no auth required)
    ListMarkets {
        /// Maximum number of markets to return
        #[arg(long, default_value = "20")]
        limit: u32,

        /// Filter markets by keyword
        #[arg(long)]
        keyword: Option<String>,

        /// Show hottest breaking events by 24h volume (excludes 5-minute rolling markets)
        #[arg(long)]
        breaking: bool,

        /// Filter by category: sports, elections, crypto
        #[arg(long, value_parser = ["sports", "elections", "crypto"])]
        category: Option<String>,
    },

    /// Get details for a specific market (no auth required)
    GetMarket {
        /// Market identifier: condition_id (0x-prefixed hex) or slug (string)
        #[arg(long)]
        market_id: String,
    },

    /// Get open positions for the active wallet (no auth required — uses public Data API)
    #[command(alias = "positions")]
    GetPositions {
        /// Wallet address to query (defaults to active onchainos wallet)
        #[arg(long, alias = "wallet")]
        address: Option<String>,
    },

    /// Show POL and USDC.e balances for the EOA wallet (and proxy wallet if initialized)
    Balance,

    /// Show current and next slot for a recurring series market (no auth required)
    GetSeries {
        /// Series identifier (e.g. btc-5m, eth-15m, btc-4h). Omit to list all.
        #[arg(long)]
        series: Option<String>,

        /// List all supported series
        #[arg(long)]
        list: bool,
    },

    /// Buy YES or NO shares in a market (signs via onchainos wallet)
    Buy {
        /// Market identifier: condition_id (0x-prefixed hex), slug, or series ID (e.g. btc-5m).
        /// Optional when --token-id is provided (fast path skips market lookup).
        #[arg(long)]
        market_id: Option<String>,

        /// Outcome to buy: "yes" or "no"
        #[arg(long)]
        outcome: String,

        /// USDC.e amount to spend (e.g. "100" = $100.00)
        #[arg(long)]
        amount: String,

        /// Limit price in [0, 1] (e.g. 0.65). Omit for market order (FOK)
        #[arg(long)]
        price: Option<f64>,

        /// Order type: GTC (resting limit) or FOK (fill-or-kill market)
        #[arg(long, default_value = "GTC")]
        order_type: String,

        /// Automatically approve USDC.e allowance before placing order
        #[arg(long)]
        approve: bool,

        /// Simulate without submitting order or approval
        #[arg(long)]
        dry_run: bool,

        /// Round up to the nearest valid order size if amount is too small to satisfy
        /// Polymarket's divisibility constraints at the given price. Without this flag
        /// the command exits with an error and the required minimum amount.
        #[arg(long)]
        round_up: bool,

        /// Maker-only: reject the order if it would immediately cross the spread (become a taker).
        /// Requires --order-type GTC. Qualifies for Polymarket maker rebates.
        #[arg(long)]
        post_only: bool,

        /// Cancel the order automatically at this Unix timestamp (seconds, UTC).
        /// Minimum 90 seconds from now. Creates a GTD (Good Till Date) order.
        #[arg(long)]
        expires: Option<u64>,

        /// Override trading mode for this order only: eoa or proxy.
        /// Does not change the stored default — use `switch-mode` for that.
        #[arg(long, value_parser = ["eoa", "proxy"])]
        mode: Option<String>,

        /// Confirm a previously gated action (reserved for future use)
        #[arg(long)]
        confirm: bool,

        /// Skip market lookup — use a known token ID directly (from get-series or get-market output).
        #[arg(long)]
        token_id: Option<String>,
    },

    /// Sell YES or NO shares in a market (signs via onchainos wallet)
    Sell {
        /// Market identifier: condition_id (0x-prefixed hex), slug, or series ID (e.g. btc-5m).
        /// Optional when --token-id is provided (fast path skips market lookup).
        #[arg(long)]
        market_id: Option<String>,

        /// Outcome to sell: "yes" or "no"
        #[arg(long)]
        outcome: String,

        /// Number of shares to sell (e.g. "250.5")
        #[arg(long)]
        shares: String,

        /// Limit price in [0, 1] (e.g. 0.65). Omit for market order (FOK)
        #[arg(long)]
        price: Option<f64>,

        /// Order type: GTC (resting limit) or FOK (fill-or-kill market)
        #[arg(long, default_value = "GTC")]
        order_type: String,

        /// Automatically approve CTF token allowance before placing order
        #[arg(long)]
        approve: bool,

        /// Simulate without submitting order or approval
        #[arg(long)]
        dry_run: bool,

        /// Maker-only: reject the order if it would immediately cross the spread (become a taker).
        /// Requires --order-type GTC. Qualifies for Polymarket maker rebates.
        #[arg(long)]
        post_only: bool,

        /// Cancel the order automatically at this Unix timestamp (seconds, UTC).
        /// Minimum 90 seconds from now. Creates a GTD (Good Till Date) order.
        #[arg(long)]
        expires: Option<u64>,

        /// Override trading mode for this order only: eoa or proxy.
        /// Does not change the stored default — use `switch-mode` for that.
        #[arg(long, value_parser = ["eoa", "proxy"])]
        mode: Option<String>,

        /// Confirm a low-price market sell that was previously gated
        #[arg(long)]
        confirm: bool,

        /// Skip market lookup — use a known token ID directly (from get-series or get-market output).
        #[arg(long)]
        token_id: Option<String>,
    },

    /// Create a Polymarket proxy wallet and switch to gasless POLY_PROXY trading mode.
    /// One-time POL gas cost; all subsequent trading is relayer-paid.
    SetupProxy {
        /// Preview the action without submitting any transaction
        #[arg(long)]
        dry_run: bool,
    },

    /// Deposit tokens into the proxy wallet via Polygon direct transfer or bridge.
    /// Requires `setup-proxy` to have been run first.
    /// Use --list to see all supported chains and tokens.
    Deposit {
        /// USD amount to deposit (e.g. "50" = $50). Always in USD — non-stablecoins are auto-converted at live price. Not required with --list.
        #[arg(long)]
        amount: Option<String>,

        /// Source chain (default: polygon). Examples: polygon, ethereum, arbitrum, base, optimism, bnb
        #[arg(long, default_value = "polygon")]
        chain: String,

        /// Token symbol to deposit (default: USDC). Examples: USDC, USDC.e, ETH, WBTC
        #[arg(long, default_value = "USDC")]
        token: String,

        /// List all supported chains and tokens, then exit
        #[arg(long)]
        list: bool,

        /// Preview the transfer without submitting
        #[arg(long)]
        dry_run: bool,
    },

    /// Withdraw USDC.e from the proxy wallet back to the EOA wallet.
    Withdraw {
        /// USDC.e amount to withdraw (e.g. "10" = $10.00)
        #[arg(long)]
        amount: String,

        /// Preview the withdrawal without submitting
        #[arg(long)]
        dry_run: bool,
    },

    /// Switch the default trading mode between EOA and POLY_PROXY.
    SwitchMode {
        /// Mode to switch to: eoa or proxy
        #[arg(long, value_parser = ["eoa", "proxy"])]
        mode: String,
    },

    /// Redeem winning outcome tokens after a market resolves (signs via onchainos wallet)
    Redeem {
        /// Market identifier: condition_id (0x-prefixed hex) or slug. Omit when using --all.
        #[arg(long)]
        market_id: Option<String>,

        /// Redeem all redeemable positions across EOA and proxy wallets in one pass
        #[arg(long)]
        all: bool,

        /// Preview the redemption call without submitting the transaction
        #[arg(long)]
        dry_run: bool,
    },

    /// Cancel a single open order by order ID (signs via onchainos wallet)
    Cancel {
        /// Order ID (0x-prefixed hash). Omit to cancel all orders.
        #[arg(long)]
        order_id: Option<String>,

        /// Cancel all orders for a specific market (by condition_id)
        #[arg(long)]
        market: Option<String>,

        /// Cancel all open orders (use with caution)
        #[arg(long)]
        all: bool,
    },

    /// List upcoming 5-minute crypto Up/Down markets on Polymarket.
    /// Supported coins: BTC, ETH, SOL, XRP, BNB, DOGE, HYPE
    #[command(name = "list-5m")]
    List5m {
        /// Coin to list markets for (BTC, ETH, SOL, XRP, BNB, DOGE, HYPE)
        #[arg(long)]
        coin: Option<String>,

        /// Number of upcoming 5-minute windows to show (default: 5, max: 20)
        #[arg(long, default_value = "5")]
        count: u32,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Quickstart(args) => {
            commands::quickstart::run(args).await
        }
        Commands::CheckAccess => {
            commands::check_access::run().await
        }
        Commands::ListMarkets { limit, keyword, breaking, category } => {
            commands::list_markets::run(limit, keyword.as_deref(), breaking, category.as_deref()).await
        }
        Commands::GetMarket { market_id } => {
            commands::get_market::run(&market_id).await
        }
        Commands::GetPositions { address } => {
            commands::get_positions::run(address.as_deref()).await
        }
        Commands::Balance => {
            commands::balance::run().await
        }
        Commands::GetSeries { series, list } => {
            commands::get_series::run(series.as_deref(), list).await
        }
        Commands::Buy {
            market_id,
            outcome,
            amount,
            price,
            order_type,
            approve,
            dry_run,
            round_up,
            post_only,
            expires,
            mode,
            confirm: _confirm,
            token_id,
        } => {
            commands::buy::run(market_id.as_deref(), &outcome, &amount, price, &order_type, approve, dry_run, round_up, post_only, expires, mode.as_deref(), token_id.as_deref()).await
        }
        Commands::Sell {
            market_id,
            outcome,
            shares,
            price,
            order_type,
            approve,
            dry_run,
            post_only,
            expires,
            mode,
            confirm: _confirm,
            token_id,
        } => {
            commands::sell::run(market_id.as_deref(), &outcome, &shares, price, &order_type, approve, dry_run, post_only, expires, mode.as_deref(), token_id.as_deref()).await
        }
        Commands::SetupProxy { dry_run } => {
            commands::setup_proxy::run(dry_run).await
        }
        Commands::Deposit { amount, chain, token, list, dry_run } => {
            commands::deposit::run(amount.as_deref(), &chain, &token, list, dry_run).await
        }
        Commands::Withdraw { amount, dry_run } => {
            commands::withdraw::run(&amount, dry_run).await
        }
        Commands::SwitchMode { mode } => {
            commands::switch_mode::run(&mode).await
        }
        Commands::Redeem { market_id, all, dry_run } => {
            if all {
                commands::redeem::run_all(dry_run).await
            } else if let Some(mid) = market_id {
                commands::redeem::run(&mid, dry_run).await
            } else {
                eprintln!("Error: provide --market-id <ID> or --all");
                std::process::exit(1);
            }
        }
        Commands::Cancel { order_id, market, all } => {
            if all {
                commands::cancel::run_cancel_all().await
            } else if let Some(oid) = order_id {
                commands::cancel::run_cancel_order(&oid).await
            } else if let Some(mkt) = market {
                commands::cancel::run_cancel_market(&mkt, None).await
            } else {
                Err(anyhow::anyhow!(
                    "Specify --order-id <id>, --market <condition_id>, or --all"
                ))
            }
        }
        Commands::List5m { coin, count } => {
            commands::list_5m::run(coin.as_deref(), count).await
        }
    };

    if let Err(e) = result {
        let err_out = serde_json::json!({
            "ok": false,
            "error": e.to_string(),
        });
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&err_out).unwrap_or_else(|_| e.to_string())
        );
        std::process::exit(1);
    }
}
