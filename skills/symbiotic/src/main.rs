mod commands;
mod config;
mod onchainos;
mod rpc;
mod api;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "symbiotic", about = "Symbiotic restaking protocol CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all Symbiotic vaults with TVL and APR
    Vaults(commands::vaults::VaultsArgs),
    /// Show your restaking positions across Symbiotic vaults
    Positions(commands::positions::PositionsArgs),
    /// Show vault APR and reward rates
    Rates(commands::rates::RatesArgs),
    /// Deposit collateral into a Symbiotic vault
    Deposit(commands::deposit::DepositArgs),
    /// Request withdrawal from a Symbiotic vault
    Withdraw(commands::withdraw::WithdrawArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Vaults(args) => commands::vaults::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Rates(args) => commands::rates::run(args).await,
        Commands::Deposit(args) => commands::deposit::run(args).await,
        Commands::Withdraw(args) => commands::withdraw::run(args).await,
    };
    match result {
        Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap()),
        Err(e) => {
            let err = serde_json::json!({"ok": false, "error": e.to_string()});
            eprintln!("{}", serde_json::to_string_pretty(&err).unwrap());
            std::process::exit(1);
        }
    }
}
