mod api;
mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    positions::PositionsArgs,
    stake::StakeArgs,
    unwrap::UnwrapArgs,
    wrap::WrapArgs,
};

#[derive(Parser)]
#[command(
    name = "etherfi",
    version,
    about = "ether.fi liquid restaking plugin for Ethereum — stake ETH, wrap/unwrap eETH/weETH"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show eETH and weETH balances, protocol APY, and exchange rate (read-only)
    Positions(PositionsArgs),
    /// Deposit ETH into LiquidityPool to receive eETH
    Stake(StakeArgs),
    /// Wrap eETH → weETH (ERC-4626 deposit)
    Wrap(WrapArgs),
    /// Unwrap weETH → eETH (ERC-4626 redeem)
    Unwrap(UnwrapArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::Wrap(args) => commands::wrap::run(args).await,
        Commands::Unwrap(args) => commands::unwrap::run(args).await,
    }
}
