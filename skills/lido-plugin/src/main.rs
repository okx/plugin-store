mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lido", version, about = "Lido liquid staking plugin for onchainos")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Stake ETH to receive stETH
    Stake(commands::stake::StakeArgs),
    /// Get current stETH staking APR
    GetApy,
    /// Get stETH balance for an address
    Balance(commands::balance::BalanceArgs),
    /// Request withdrawal of stETH for ETH
    RequestWithdrawal(commands::request_withdrawal::RequestWithdrawalArgs),
    /// Get pending withdrawal requests for an address
    GetWithdrawals(commands::get_withdrawals::GetWithdrawalsArgs),
    /// Claim finalized withdrawal(s)
    ClaimWithdrawal(commands::claim_withdrawal::ClaimWithdrawalArgs),
    /// Wrap stETH into wstETH (ERC-20 yield-bearing token)
    Wrap(commands::wrap::WrapArgs),
    /// Unwrap wstETH back to stETH
    Unwrap(commands::unwrap::UnwrapArgs),
    /// Check wallet state and get guided next steps
    Quickstart,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::GetApy => commands::get_apy::run().await,
        Commands::Balance(args) => commands::balance::run(args).await,
        Commands::RequestWithdrawal(args) => commands::request_withdrawal::run(args).await,
        Commands::GetWithdrawals(args) => commands::get_withdrawals::run(args).await,
        Commands::ClaimWithdrawal(args) => commands::claim_withdrawal::run(args).await,
        Commands::Wrap(args) => commands::wrap::run(args).await,
        Commands::Unwrap(args) => commands::unwrap::run(args).await,
        Commands::Quickstart => commands::quickstart::run().await,
    }
}
