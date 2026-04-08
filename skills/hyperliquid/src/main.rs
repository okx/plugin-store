mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;
mod signing;

use clap::{Parser, Subcommand};
use commands::{
    cancel::CancelArgs,
    deposit::DepositArgs,
    order::OrderArgs,
    positions::PositionsArgs,
    prices::PricesArgs,
};

#[derive(Parser)]
#[command(
    name = "hyperliquid",
    version,
    about = "Hyperliquid on-chain perpetuals DEX plugin — trade perps, check positions, get prices, deposit USDC"
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
    /// Place a market or limit perpetual order (requires --confirm to execute)
    Order(OrderArgs),
    /// Cancel an open order by order ID (requires --confirm to execute)
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
        Commands::Cancel(args) => commands::cancel::run(args).await,
        Commands::Deposit(args) => commands::deposit::run(args).await,
    }
}
