mod api;
mod commands;
mod config;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    close_position::ClosePositionArgs,
    markets::MarketsArgs,
    open_position::OpenPositionArgs,
    positions::PositionsArgs,
    quote::QuoteArgs,
};

#[derive(Parser)]
#[command(
    name = "bluefin",
    version,
    about = "Bluefin DEX plugin — spot and perpetuals trading on Sui"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available trading markets with current prices and 24h stats
    Markets(MarketsArgs),

    /// Show open perpetual positions for a wallet address
    Positions(PositionsArgs),

    /// Get orderbook quote for a given market and size
    Quote(QuoteArgs),

    /// Preview or open a perpetual position (requires --confirm to submit)
    OpenPosition(OpenPositionArgs),

    /// Preview or close a perpetual position (requires --confirm to submit)
    ClosePosition(ClosePositionArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Markets(args) => commands::markets::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Quote(args) => commands::quote::run(args).await,
        Commands::OpenPosition(args) => commands::open_position::run(args).await,
        Commands::ClosePosition(args) => commands::close_position::run(args).await,
    }
}
