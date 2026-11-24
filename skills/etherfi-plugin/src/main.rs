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
    unstake::UnstakeArgs,
    unwrap::UnwrapArgs,
    wrap::WrapArgs,
};
use clap::Args;

#[derive(Parser)]
#[command(
    name = "etherfi",
    version,
    about = "ether.fi liquid restaking plugin for Ethereum — stake ETH, wrap/unwrap eETH/weETH, unstake eETH"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args)]
struct QuickstartArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    from: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show eETH and weETH balances, protocol APY, and exchange rate (read-only)
    Positions(PositionsArgs),
    /// Deposit ETH into LiquidityPool to receive eETH
    Stake(StakeArgs),
    /// Request eETH withdrawal (Step 1) or claim finalized ETH (Step 2 with --claim --token-id)
    Unstake(UnstakeArgs),
    /// Wrap eETH → weETH (ERC-4626 deposit)
    Wrap(WrapArgs),
    /// Unwrap weETH → eETH (ERC-4626 redeem)
    Unwrap(UnwrapArgs),
    /// Check wallet state and get personalised onboarding steps
    Quickstart(QuickstartArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::Unstake(args) => commands::unstake::run(args).await,
        Commands::Wrap(args) => commands::wrap::run(args).await,
        Commands::Unwrap(args) => commands::unwrap::run(args).await,
        Commands::Quickstart(args) => commands::quickstart::run(args.from.as_deref()).await,
    }
}
