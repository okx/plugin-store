mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sushiswap-v3-plugin",
    version,
    about = "Swap tokens and manage concentrated liquidity positions on SushiSwap V3"
)]
struct Cli {
    /// Chain ID (1=Ethereum, 42161=Arbitrum, 8453=Base, 137=Polygon, 10=Optimism)
    #[arg(long, global = true, default_value = "42161")]
    chain: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a swap quote without executing
    Quote(commands::quote::QuoteArgs),
    /// Swap tokens through SushiSwap V3
    Swap(commands::swap::SwapArgs),
    /// List available V3 pools for a token pair
    Pools(commands::pools::PoolsArgs),
    /// List your concentrated liquidity positions
    Positions(commands::positions::PositionsArgs),
    /// Open a new concentrated liquidity position (mint NFT)
    MintPosition(commands::mint_position::MintPositionArgs),
    /// Remove liquidity from a position (decreaseLiquidity + collect)
    RemoveLiquidity(commands::remove_liquidity::RemoveLiquidityArgs),
    /// Collect uncollected trading fees from a position
    CollectFees(commands::collect_fees::CollectFeesArgs),
    /// Permanently destroy a zero-liquidity position NFT
    BurnPosition(commands::burn_position::BurnPositionArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let chain = cli.chain;
    match cli.command {
        Commands::Quote(a)           => commands::quote::run(a, chain).await,
        Commands::Swap(a)            => commands::swap::run(a, chain).await,
        Commands::Pools(a)           => commands::pools::run(a, chain).await,
        Commands::Positions(a)       => commands::positions::run(a, chain).await,
        Commands::MintPosition(a)    => commands::mint_position::run(a, chain).await,
        Commands::RemoveLiquidity(a) => commands::remove_liquidity::run(a, chain).await,
        Commands::CollectFees(a)     => commands::collect_fees::run(a, chain).await,
        Commands::BurnPosition(a)    => commands::burn_position::run(a, chain).await,
    }
}
