mod api;
mod commands;
mod config;
mod rpc;

use anyhow::Result;
use clap::{Parser, Subcommand};

use commands::{
    borrow::BorrowArgs,
    markets::MarketsArgs,
    positions::PositionsArgs,
    repay::RepayArgs,
    supply::SupplyArgs,
    withdraw::WithdrawArgs,
};

#[derive(Parser, Debug)]
#[command(
    name = "scallop-lend",
    version = "0.1.0",
    about = "Scallop Lend — Supply and borrow on Scallop (Sui blockchain)",
    long_about = "CLI plugin for Scallop Lend, the leading lending protocol on Sui.\n\
                  Read commands query Sui mainnet via JSON-RPC.\n\
                  Write commands (supply/withdraw/borrow/repay) are preview-only\n\
                  until onchainos Sui support is available."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List lending markets with APY and utilization
    Markets(MarketsArgs),

    /// Show supply/borrow positions for a wallet
    Positions(PositionsArgs),

    /// Supply assets to Scallop Lend (dry-run by default)
    Supply(SupplyArgs),

    /// Withdraw supplied assets (dry-run by default)
    Withdraw(WithdrawArgs),

    /// Borrow assets against collateral (dry-run by default)
    Borrow(BorrowArgs),

    /// Repay a loan (dry-run by default)
    Repay(RepayArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Markets(args) => commands::markets::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Supply(args) => commands::supply::run(args).await,
        Commands::Withdraw(args) => commands::withdraw::run(args).await,
        Commands::Borrow(args) => commands::borrow::run(args).await,
        Commands::Repay(args) => commands::repay::run(args).await,
    }
}
