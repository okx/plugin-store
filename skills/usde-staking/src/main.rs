mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "usde-staking",
    about = "Ethena sUSDe staking plugin — stake USDe to earn yield via ERC-4626 vault"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get current sUSDe staking yield and exchange rate
    GetRates,
    /// Get sUSDe position and pending unstake status
    GetPositions(commands::get_positions::GetPositionsArgs),
    /// Stake USDe to receive sUSDe (approve + ERC-4626 deposit)
    Stake(commands::stake::StakeArgs),
    /// Initiate cooldown to unstake sUSDe (cooldown-gated)
    RequestUnstake(commands::request_unstake::RequestUnstakeArgs),
    /// Claim USDe after cooldown period completes
    ClaimUnstake(commands::claim_unstake::ClaimUnstakeArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::GetRates => commands::get_rates::run().await,
        Commands::GetPositions(args) => commands::get_positions::run(args).await,
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::RequestUnstake(args) => commands::request_unstake::run(args).await,
        Commands::ClaimUnstake(args) => commands::claim_unstake::run(args).await,
    }
}
