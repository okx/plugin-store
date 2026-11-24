mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "aerodrome-slipstream-plugin",
    version,
    about = "Swap tokens and manage concentrated liquidity positions on Aerodrome Slipstream (CLMM) on Base"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a swap quote without executing
    Quote(commands::quote::QuoteArgs),
    /// Swap tokens through Aerodrome Slipstream CL pools
    Swap(commands::swap::SwapArgs),
    /// List available CL pools for a token pair
    Pools(commands::pools::PoolsArgs),
    /// Get the current price for a token pair
    Prices(commands::prices::PricesArgs),
    /// List your concentrated liquidity positions (NFT positions)
    Positions(commands::positions::PositionsArgs),
    /// Open a new concentrated liquidity position
    MintPosition(commands::mint_position::MintPositionArgs),
    /// Add liquidity to an existing position
    AddLiquidity(commands::add_liquidity::AddLiquidityArgs),
    /// Burn (permanently destroy) a zero-liquidity position NFT
    BurnPosition(commands::burn_position::BurnPositionArgs),
    /// Remove liquidity from a position (decreaseLiquidity + collect)
    RemoveLiquidity(commands::remove_liquidity::RemoveLiquidityArgs),
    /// Collect uncollected trading fees from a position
    CollectFees(commands::collect_fees::CollectFeesArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quote(a)           => commands::quote::run(a).await,
        Commands::Swap(a)            => commands::swap::run(a).await,
        Commands::Pools(a)           => commands::pools::run(a).await,
        Commands::Prices(a)          => commands::prices::run(a).await,
        Commands::Positions(a)       => commands::positions::run(a).await,
        Commands::MintPosition(a)    => commands::mint_position::run(a).await,
        Commands::AddLiquidity(a)    => commands::add_liquidity::run(a).await,
        Commands::BurnPosition(a)    => commands::burn_position::run(a).await,
        Commands::RemoveLiquidity(a) => commands::remove_liquidity::run(a).await,
        Commands::CollectFees(a)     => commands::collect_fees::run(a).await,
    }
}
