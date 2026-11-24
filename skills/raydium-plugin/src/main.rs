mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "raydium",
    version,
    about = "Raydium AMM plugin — swap, price, and pool queries on Solana"
)]
struct Cli {
    /// Simulate without broadcasting on-chain (no onchainos call)
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a swap quote (estimate output amount, price impact, route)
    GetSwapQuote(commands::get_swap_quote::GetSwapQuoteArgs),

    /// Compute price ratio between two tokens
    GetPrice(commands::get_price::GetPriceArgs),

    /// Get USD price for one or more token mints
    GetTokenPrice(commands::get_token_price::GetTokenPriceArgs),

    /// Query pool info by pool IDs or token mint addresses
    GetPools(commands::get_pools::GetPoolsArgs),

    /// List pools with pagination and sorting
    GetPoolList(commands::get_pool_list::GetPoolListArgs),

    /// Execute a token swap on Raydium (requires onchainos login)
    Swap(commands::swap::SwapArgs),

    /// Check wallet state and get guided next steps
    Quickstart,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GetSwapQuote(args) => commands::get_swap_quote::execute(&args).await?,
        Commands::GetPrice(args) => commands::get_price::execute(&args).await?,
        Commands::GetTokenPrice(args) => commands::get_token_price::execute(&args).await?,
        Commands::GetPools(args) => commands::get_pools::execute(&args).await?,
        Commands::GetPoolList(args) => commands::get_pool_list::execute(&args).await?,
        Commands::Swap(args) => commands::swap::execute(&args, cli.dry_run).await?,
        Commands::Quickstart => commands::quickstart::run().await?,
    }

    Ok(())
}
