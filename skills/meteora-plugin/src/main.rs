mod api;
mod commands;
mod config;
mod meteora_ix;
mod onchainos;
mod solana_rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "meteora-plugin",
    about = "Meteora DLMM plugin — query pools, get quotes, check positions, manage liquidity, and execute swaps on Solana",
    version = env!("CARGO_PKG_VERSION")
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

    /// Check wallet balances and get a recommended deposit command for a pool
    Quickstart(commands::quickstart::QuickstartArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::GetPools(args) => commands::get_pools::execute(args).await?,
        Commands::GetPoolDetail(args) => commands::get_pool_detail::execute(args).await?,
        Commands::GetSwapQuote(args) => commands::get_swap_quote::execute(args).await?,
        Commands::GetUserPositions(args) => commands::get_user_positions::execute(args).await?,
        Commands::Swap(args) => commands::swap::execute(args, cli.confirm).await?,
        Commands::AddLiquidity(args) => commands::add_liquidity::execute(args, cli.confirm).await?,
        Commands::RemoveLiquidity(args) => commands::remove_liquidity::execute(args, cli.confirm).await?,
        Commands::Quickstart(args) => commands::quickstart::execute(args).await?,
    }

    Ok(())
}
