mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "stakestone", about = "StakeStone liquid staking plugin for onchainos")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Stake ETH to receive STONE liquid staking tokens
    Stake(commands::stake::StakeArgs),
    /// Request withdrawal of STONE tokens back to ETH queue
    RequestWithdraw(commands::request_withdraw::RequestWithdrawArgs),
    /// Cancel a pending withdrawal request
    CancelWithdraw(commands::cancel_withdraw::CancelWithdrawArgs),
    /// Get current STONE exchange rate and vault info
    GetRate,
    /// Get STONE position and pending withdrawal for an address
    GetPosition(commands::get_position::GetPositionArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::RequestWithdraw(args) => commands::request_withdraw::run(args).await,
        Commands::CancelWithdraw(args) => commands::cancel_withdraw::run(args).await,
        Commands::GetRate => commands::get_rate::run(),
        Commands::GetPosition(args) => commands::get_position::run(args),
    }
}
