mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};
use commands::{get_pools, get_quote, quickstart, swap};

#[derive(Parser)]
#[command(
    name = "orca-plugin",
    version,
    about = "Orca Whirlpools DEX plugin — swap tokens and query liquidity pools on Solana"
)]
struct Cli {
    /// Execute the transaction on-chain (without this flag, the command previews only)
    #[arg(long, global = true)]
    confirm: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List Orca Whirlpool pools for a token pair
    GetPools(get_pools::GetPoolsArgs),

    /// Get a swap quote for a token pair on Orca
    GetQuote(get_quote::GetQuoteArgs),

    /// Execute a token swap on Orca via onchainos
    Swap(swap::SwapArgs),

    /// Check wallet assets and get a recommended next step for Orca
    Quickstart,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::GetPools(args) => get_pools::execute(args).await?,
        Commands::GetQuote(args) => get_quote::execute(args).await?,
        Commands::Swap(args) => swap::execute(args, cli.confirm).await?,
        Commands::Quickstart => quickstart::run(cli.confirm).await?,
    }

    Ok(())
}
