mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "frax-ether",
    about = "Frax Ether liquid staking — stake ETH to frxETH, frxETH to sfrxETH"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Stake ETH to receive frxETH (via frxETHMinter.submit)
    Stake(commands::stake::StakeArgs),
    /// Stake frxETH to receive yield-bearing sfrxETH (ERC-4626 deposit)
    StakeFrx(commands::stake_frx::StakeFrxArgs),
    /// Redeem sfrxETH back to frxETH (ERC-4626 redeem)
    Unstake(commands::unstake::UnstakeArgs),
    /// Get current sfrxETH APR and exchange rate
    Rates,
    /// Query frxETH and sfrxETH positions for a wallet
    Positions(commands::positions::PositionsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::StakeFrx(args) => commands::stake_frx::run(args).await,
        Commands::Unstake(args) => commands::unstake::run(args).await,
        Commands::Rates => commands::rates::run().await,
        Commands::Positions(args) => commands::positions::run(args).await,
    }
}
