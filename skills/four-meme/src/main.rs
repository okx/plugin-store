mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "four-meme",
    about = "Buy and sell meme tokens on Four.meme bonding curve launchpad on BNB Chain",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List supported base tokens and platform config from Four.meme
    Tokens(commands::tokens::TokensArgs),

    /// Get token details, price, market cap, and bonding curve progress
    Info(commands::info::InfoArgs),

    /// Buy a meme token from the bonding curve using BNB
    Buy(commands::buy::BuyArgs),

    /// Sell meme tokens back to the bonding curve for BNB
    Sell(commands::sell::SellArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Tokens(args) => commands::tokens::run(args).await?,
        Commands::Info(args) => commands::info::run(args).await?,
        Commands::Buy(args) => commands::buy::run(args).await?,
        Commands::Sell(args) => commands::sell::run(args).await?,
    }

    Ok(())
}
