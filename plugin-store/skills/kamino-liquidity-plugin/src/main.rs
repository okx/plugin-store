mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kamino-liquidity",
    about = "Kamino Liquidity plugin — deposit into and withdraw from Kamino KVault earn vaults on Solana",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show wallet status, balances, and suggested first command
    Quickstart(commands::quickstart::QuickstartArgs),
    /// List all Kamino KVault earn vaults
    Vaults(commands::vaults::VaultsArgs),
    /// Query your Kamino KVault positions (share balances)
    Positions(commands::positions::PositionsArgs),
    /// Deposit tokens into a Kamino KVault
    Deposit(commands::deposit::DepositArgs),
    /// Withdraw shares from a Kamino KVault
    Withdraw(commands::withdraw::WithdrawArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quickstart(args) => commands::quickstart::run(args).await,
        Commands::Vaults(args) => commands::vaults::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Deposit(args) => commands::deposit::run(args).await,
        Commands::Withdraw(args) => commands::withdraw::run(args).await,
    }
}
