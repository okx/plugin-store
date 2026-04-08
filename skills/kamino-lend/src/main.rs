mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kamino-lend", about = "Kamino Lend plugin — supply, borrow, and manage positions on Kamino lending markets (Solana)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List Kamino lending markets and their interest rates
    Markets(commands::markets::MarketsArgs),
    /// Query user lending positions (obligations) on Kamino
    Positions(commands::positions::PositionsArgs),
    /// Supply (deposit) assets into a Kamino lending market
    Supply(commands::supply::SupplyArgs),
    /// Withdraw assets from a Kamino lending market
    Withdraw(commands::withdraw::WithdrawArgs),
    /// Borrow assets from a Kamino lending market (dry-run supported)
    Borrow(commands::borrow::BorrowArgs),
    /// Repay borrowed assets on Kamino (dry-run supported)
    Repay(commands::repay::RepayArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Markets(args) => commands::markets::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Supply(args) => commands::supply::run(args).await,
        Commands::Withdraw(args) => commands::withdraw::run(args).await,
        Commands::Borrow(args) => commands::borrow::run(args).await,
        Commands::Repay(args) => commands::repay::run(args).await,
    }
}
