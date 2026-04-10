mod api;
mod auth;
mod commands;
mod config;
mod onchainos;
mod sanitize;
mod signing;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "polymarket",
    version = "0.2.0",
    about = "Trade prediction markets on Polymarket — buy and sell YES/NO outcome tokens on Polygon (chain 137)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
        } => {
            commands::buy::run(&market_id, &outcome, &amount, price, &order_type, approve, dry_run).await
        }
        Commands::Sell {
            market_id,
            outcome,
            shares,
            price,
            order_type,
            approve,
            dry_run,
        } => {
            commands::sell::run(&market_id, &outcome, &shares, price, &order_type, approve, dry_run).await
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
