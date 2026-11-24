use clap::{Parser, Subcommand};

mod commands;
mod config;
mod onchainos;
mod rpc;

#[derive(Parser)]
#[command(
    name = "aerodrome-amm-plugin",
    version = env!("CARGO_PKG_VERSION"),
    about = "Swap tokens and provide liquidity on Aerodrome AMM (volatile/stable pools) on Base"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Quote a swap without executing
    Quote(commands::quote::QuoteArgs),
    /// Swap tokens through volatile or stable pools
    Swap(commands::swap::SwapArgs),
    /// List AMM pools for a token pair
    Pools(commands::pools::PoolsArgs),
    /// Get token prices from AMM pool reserves
    Prices(commands::prices::PricesArgs),
    /// Show LP token positions for the active wallet
    Positions(commands::positions::PositionsArgs),
    /// Add liquidity to a pool and receive LP tokens
    AddLiquidity(commands::add_liquidity::AddLiquidityArgs),
    /// Remove liquidity by burning LP tokens
    RemoveLiquidity(commands::remove_liquidity::RemoveLiquidityArgs),
    /// Claim accrued trading fees from an LP position
    ClaimFees(commands::claim_fees::ClaimFeesArgs),
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
        Commands::AddLiquidity(a)    => commands::add_liquidity::run(a).await,
        Commands::RemoveLiquidity(a) => commands::remove_liquidity::run(a).await,
        Commands::ClaimFees(a)       => commands::claim_fees::run(a).await,
    }
}
