mod commands;
mod config;
mod contracts;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "rocket-pool",
    about = "Rocket Pool decentralised ETH liquid staking plugin for onchainos"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get current ETH/rETH exchange rate
    Rate(commands::rate::RateArgs),

    /// Get current rETH staking APY
    Apy(commands::apy::ApyArgs),

    /// Get Rocket Pool protocol stats (TVL, nodes, minipools)
    Stats(commands::stats::StatsArgs),

    /// Get rETH position for a wallet address
    Positions(commands::positions::PositionsArgs),

    /// Stake ETH to receive rETH (deposit into RocketDepositPool)
    Stake(commands::stake::StakeArgs),

    /// Unstake: burn rETH to receive ETH
    Unstake(commands::unstake::UnstakeArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Rate(args) => commands::rate::run(args).await,
        Commands::Apy(args) => commands::apy::run(args).await,
        Commands::Stats(args) => commands::stats::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::Unstake(args) => commands::unstake::run(args).await,
    }
}
