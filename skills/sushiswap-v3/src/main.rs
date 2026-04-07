mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    add_liquidity::AddLiquidityArgs,
    collect_fees::CollectFeesArgs,
    get_pools::GetPoolsArgs,
    get_positions::GetPositionsArgs,
    quote::QuoteArgs,
    remove_liquidity::RemoveLiquidityArgs,
    swap::SwapArgs,
};

#[derive(Parser)]
#[command(name = "sushiswap-v3", version, about = "SushiSwap V3 Plugin — Swap and manage concentrated liquidity on 38+ EVM chains")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a swap quote via QuoterV2 eth_call (no transaction)
    Quote(QuoteArgs),
    /// Swap tokens via SwapRouter exactInputSingle
    Swap(SwapArgs),
    /// Add concentrated liquidity (mint a new position NFT)
    AddLiquidity(AddLiquidityArgs),
    /// Remove concentrated liquidity from a position
    RemoveLiquidity(RemoveLiquidityArgs),
    /// Collect accumulated fees from a position
    CollectFees(CollectFeesArgs),
    /// List all LP positions for a wallet
    GetPositions(GetPositionsArgs),
    /// Query pool addresses from the Factory for a token pair
    GetPools(GetPoolsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quote(args) => commands::quote::run(args).await,
        Commands::Swap(args) => commands::swap::run(args).await,
        Commands::AddLiquidity(args) => commands::add_liquidity::run(args).await,
        Commands::RemoveLiquidity(args) => commands::remove_liquidity::run(args).await,
        Commands::CollectFees(args) => commands::collect_fees::run(args).await,
        Commands::GetPositions(args) => commands::get_positions::run(args).await,
        Commands::GetPools(args) => commands::get_pools::run(args).await,
    }
}
