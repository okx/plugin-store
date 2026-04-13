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
    },

    /// Get details for a specific market (no auth required)
    GetMarket {
        /// Market identifier: condition_id (0x-prefixed hex) or slug (string)
        #[arg(long)]
        market_id: String,
    },

    /// Get open positions for the active wallet (no auth required — uses public Data API)
    GetPositions {
        /// Wallet address to query (defaults to active onchainos wallet)
        #[arg(long, alias = "wallet")]
        address: Option<String>,
    },

    /// Buy YES or NO shares in a market (signs via onchainos wallet)
    Buy {
        /// Market identifier: condition_id (0x-prefixed hex) or slug
        #[arg(long)]
        market_id: String,

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

        /// Confirm a previously gated action (reserved for future use)
        #[arg(long)]
        confirm: bool,

        /// Skip market lookup — use a known token ID directly (from get-series or get-market output).
        /// Faster for repeated trades: resolves condition_id and neg_risk from the order book.
        #[arg(long)]
        token_id: Option<String>,
    },

    /// Sell YES or NO shares in a market (signs via onchainos wallet)
    Sell {
        /// Market identifier: condition_id (0x-prefixed hex) or slug
        #[arg(long)]
        market_id: String,

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

        /// Confirm a low-price market sell that was previously gated
        #[arg(long)]
        confirm: bool,

        /// Skip market lookup — use a known token ID directly (from get-series or get-market output).
        /// Faster for repeated trades: resolves condition_id and neg_risk from the order book.
        #[arg(long)]
        token_id: Option<String>,
    },

    /// Redeem winning outcome tokens after a market resolves (signs via onchainos wallet)
    Redeem {
        /// Market identifier: condition_id (0x-prefixed hex) or slug
        #[arg(long)]
        market_id: String,

        /// Preview the redemption call without submitting the transaction
        #[arg(long)]
        dry_run: bool,
    },

    /// Get current and next slot for a recurring series market (e.g. btc-5m, eth-5m)
    GetSeries {
        /// Series ID: btc-5m, eth-5m, sol-5m, xrp-5m (or just btc, eth, sol, xrp)
        #[arg(long)]
        series: Option<String>,

        /// List all supported series and exit
        #[arg(long)]
        list: bool,
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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::CheckAccess => {
            commands::check_access::run().await
        }
        Commands::ListMarkets { limit, keyword } => {
            commands::list_markets::run(limit, keyword.as_deref()).await
        }
        Commands::GetMarket { market_id } => {
            commands::get_market::run(&market_id).await
        }
        Commands::GetPositions { address } => {
            commands::get_positions::run(address.as_deref()).await
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
            confirm: _confirm,
            token_id,
        } => {
            commands::buy::run(&market_id, &outcome, &amount, price, &order_type, approve, dry_run, round_up, post_only, expires, token_id.as_deref()).await
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
            confirm: _confirm,
            token_id,
        } => {
            commands::sell::run(&market_id, &outcome, &shares, price, &order_type, approve, dry_run, post_only, expires, token_id.as_deref()).await
        }
        Commands::GetSeries { series, list } => {
            commands::get_series::run(series.as_deref(), list).await
        }
        Commands::Redeem { market_id, dry_run } => {
            commands::redeem::run(&market_id, dry_run).await
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
