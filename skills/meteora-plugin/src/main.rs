mod api;
mod commands;
mod config;
mod meteora_ix;
mod onchainos;
mod solana_rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "meteora",
    about = "Meteora DLMM plugin — query pools, get quotes, check positions, manage liquidity, and execute swaps on Solana",
    version = "0.2.0"
)]
struct Cli {
    /// Simulate without broadcasting (dry run)
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List and search Meteora DLMM liquidity pools
    GetPools(commands::get_pools::GetPoolsArgs),

    /// Get detailed info for a specific pool
    GetPoolDetail(commands::get_pool_detail::GetPoolDetailArgs),

    /// Get a swap quote for a token pair
    GetSwapQuote(commands::get_swap_quote::GetSwapQuoteArgs),

    /// Get user LP positions in Meteora DLMM pools
    GetUserPositions(commands::get_user_positions::GetUserPositionsArgs),

    /// Execute a token swap via Meteora DLMM
    Swap(commands::swap::SwapArgs),

    /// Add liquidity to a Meteora DLMM pool (SpotBalanced strategy)
    AddLiquidity(commands::add_liquidity::AddLiquidityArgs),

    /// Remove liquidity from a Meteora DLMM position
    RemoveLiquidity(commands::remove_liquidity::RemoveLiquidityArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::GetPools(args) => commands::get_pools::execute(args).await?,
        Commands::GetPoolDetail(args) => commands::get_pool_detail::execute(args).await?,
        Commands::GetSwapQuote(args) => commands::get_swap_quote::execute(args).await?,
        Commands::GetUserPositions(args) => commands::get_user_positions::execute(args).await?,
        Commands::Swap(args) => commands::swap::execute(args, cli.dry_run).await?,
        Commands::AddLiquidity(args) => commands::add_liquidity::execute(args, cli.dry_run).await?,
        Commands::RemoveLiquidity(args) => commands::remove_liquidity::execute(args, cli.dry_run).await?,
    }

    Ok(())
}
