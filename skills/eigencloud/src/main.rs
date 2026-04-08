mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    delegate::DelegateArgs,
    deposit::DepositArgs,
    positions::PositionsArgs,
    queue_withdraw::QueueWithdrawArgs,
    strategies::StrategiesArgs,
};

#[derive(Parser)]
#[command(
    name = "eigencloud",
    version,
    about = "EigenCloud (EigenLayer) restaking plugin — deposit LSTs, delegate to operators, queue withdrawals"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available EigenLayer LST strategies and their total shares (TVL)
    Strategies(StrategiesArgs),
    /// Show restaking positions (shares) for a wallet across all strategies
    Positions(PositionsArgs),
    /// Deposit an LST token into an EigenLayer strategy to earn restaking rewards
    Deposit(DepositArgs),
    /// Delegate restaked shares to an EigenLayer operator (AVS)
    Delegate(DelegateArgs),
    /// Queue a withdrawal of shares from a strategy (7-day delay applies)
    QueueWithdraw(QueueWithdrawArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Strategies(args) => commands::strategies::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Deposit(args) => commands::deposit::run(args).await,
        Commands::Delegate(args) => commands::delegate::run(args).await,
        Commands::QueueWithdraw(args) => commands::queue_withdraw::run(args).await,
    }
}
