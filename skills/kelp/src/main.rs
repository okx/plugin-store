mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kelp", about = "Kelp DAO rsETH liquid restaking plugin for onchainos")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get current rsETH APY / staking yield
    Apy,
    /// Get rsETH/ETH exchange rates from LRTOracle
    Rates(commands::rates::RatesArgs),
    /// Get rsETH positions and underlying ETH value for an address
    Positions(commands::positions::PositionsArgs),
    /// Stake ETH to receive rsETH (via LRTDepositPool)
    Stake(commands::stake::StakeArgs),
    /// Initiate rsETH withdrawal (via LRTWithdrawalManager)
    Unstake(commands::unstake::UnstakeArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Apy => commands::apy::run().await,
        Commands::Rates(args) => commands::rates::run(args.chain).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::Unstake(args) => commands::unstake::run(args).await,
    }
}
