use clap::{Parser, Subcommand};

mod abi;
mod chain;
mod onchainos;
mod strategies;
mod commands;

#[derive(Parser)]
#[command(name = "eigencloud-plugin", about = "Restake LSTs on EigenLayer to earn AVS operator yield", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List supported LST strategies
    Strategies(commands::strategies::StrategiesArgs),
    /// Show your current restaking positions and delegation status
    Positions(commands::positions::PositionsArgs),
    /// Restake an LST into EigenLayer (approve + depositIntoStrategy)
    Stake(commands::stake::StakeArgs),
    /// Delegate restaked funds to an EigenLayer operator
    Delegate(commands::delegate::DelegateArgs),
    /// Undelegate from current operator (queues all shares for withdrawal)
    Undelegate(commands::undelegate::UndelegateArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Strategies(a) => commands::strategies::run(a).await,
        Commands::Positions(a)  => commands::positions::run(a).await,
        Commands::Stake(a)      => commands::stake::run(a).await,
        Commands::Delegate(a)   => commands::delegate::run(a).await,
        Commands::Undelegate(a) => commands::undelegate::run(a).await,
    }
}
