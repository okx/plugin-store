mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    add_liquidity::AddLiquidityArgs,
    claim_rewards::ClaimRewardsArgs,
    pools::PoolsArgs,
    positions::PositionsArgs,
    quote::QuoteArgs,
    remove_liquidity::RemoveLiquidityArgs,
    swap::SwapArgs,
};

#[derive(Parser)]
#[command(name = "velodrome-v2", version, about = "Velodrome V2 AMM (volatile/stable pools) Plugin for Optimism")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a swap quote via Router.getAmountsOut (no transaction)
    Quote(QuoteArgs),
    /// Swap tokens via classic AMM Router
    Swap(SwapArgs),
    /// List classic AMM pools from PoolFactory
    Pools(PoolsArgs),
    /// Show LP positions (ERC-20 LP token balances) for a wallet
    Positions(PositionsArgs),
    /// Add liquidity to a classic AMM pool
    AddLiquidity(AddLiquidityArgs),
    /// Remove liquidity from a classic AMM pool
    RemoveLiquidity(RemoveLiquidityArgs),
    /// Claim VELO gauge rewards from a pool gauge
    ClaimRewards(ClaimRewardsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quote(args) => commands::quote::run(args).await,
        Commands::Swap(args) => commands::swap::run(args).await,
        Commands::Pools(args) => commands::pools::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::AddLiquidity(args) => commands::add_liquidity::run(args).await,
        Commands::RemoveLiquidity(args) => commands::remove_liquidity::run(args).await,
        Commands::ClaimRewards(args) => commands::claim_rewards::run(args).await,
    }
}
