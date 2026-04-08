mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;
mod signing;

use clap::{Parser, Subcommand};
use commands::{
    cancel::CancelArgs,
    close::CloseArgs,
    deposit::DepositArgs,
    order::OrderArgs,
    positions::PositionsArgs,
    prices::PricesArgs,
    tpsl::TpslArgs,
};

#[derive(Parser)]
#[command(
    name = "hyperliquid",
    version,
    about = "Hyperliquid on-chain perpetuals DEX plugin — trade perps, set TP/SL, close positions, check prices, deposit USDC"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show open perpetual positions, unrealized PnL, and margin summary
    Positions(PositionsArgs),
    /// Get current mid prices for all markets or a specific coin
    Prices(PricesArgs),
    /// Place a market or limit order; optionally attach TP/SL bracket (requires --confirm)
    Order(OrderArgs),
    /// Market-close an open position in one command (requires --confirm)
    Close(CloseArgs),
    /// Set stop-loss and/or take-profit on an existing position (requires --confirm)
    Tpsl(TpslArgs),
    /// Cancel an open order by order ID (requires --confirm)
    Cancel(CancelArgs),
    /// Deposit USDC from Arbitrum to Hyperliquid via the official bridge
    Deposit(DepositArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Prices(args) => commands::prices::run(args).await,
        Commands::Order(args) => commands::order::run(args).await,
        Commands::Close(args) => commands::close::run(args).await,
        Commands::Tpsl(args) => commands::tpsl::run(args).await,
        Commands::Cancel(args) => commands::cancel::run(args).await,
        Commands::Deposit(args) => commands::deposit::run(args).await,
    }
}
