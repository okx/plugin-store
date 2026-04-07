mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "archimedes",
    about = "Archimedes Finance V2 -- deposit into ERC4626 protected yield vaults on Ethereum",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List known Archimedes V2 vault addresses with name, underlying asset, and TVL
    Vaults(commands::vaults::VaultsArgs),

    /// Show wallet's share balance and underlying value in each vault
    Positions(commands::positions::PositionsArgs),

    /// Deposit underlying asset into an Archimedes vault (approve + deposit)
    Deposit(commands::deposit::DepositArgs),

    /// Withdraw underlying assets from a vault by specifying asset amount
    Withdraw(commands::withdraw::WithdrawArgs),

    /// Redeem vault shares for underlying assets (full or partial exit)
    Redeem(commands::redeem::RedeemArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Vaults(args) => commands::vaults::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Deposit(args) => commands::deposit::run(args).await,
        Commands::Withdraw(args) => commands::withdraw::run(args).await,
        Commands::Redeem(args) => commands::redeem::run(args).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
