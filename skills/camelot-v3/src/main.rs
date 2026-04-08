mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "camelot-v3", about = "Camelot V3 DEX plugin (Algebra V1 fork on Arbitrum)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a price quote for a token swap (no gas)
    Quote(commands::quote::QuoteArgs),
    /// Execute a token swap on Camelot V3
    Swap(commands::swap::SwapArgs),
    /// List your Camelot V3 LP positions
    Positions(commands::positions::PositionsArgs),
    /// Add concentrated liquidity to a Camelot V3 pool
    AddLiquidity(commands::add_liquidity::AddLiquidityArgs),
    /// Remove liquidity from a Camelot V3 position
    RemoveLiquidity(commands::remove_liquidity::RemoveLiquidityArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quote(args) => commands::quote::run(args).await?,
        Commands::Swap(args) => commands::swap::run(args).await?,
        Commands::Positions(args) => commands::positions::run(args).await?,
        Commands::AddLiquidity(args) => commands::add_liquidity::run(args).await?,
        Commands::RemoveLiquidity(args) => commands::remove_liquidity::run(args).await?,
    }
    Ok(())
}
