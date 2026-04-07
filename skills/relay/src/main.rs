mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "relay", about = "Relay cross-chain bridge plugin for onchainos")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all supported chains
    Chains(commands::chains::ChainsArgs),
    /// List supported currencies/tokens on a chain
    Currencies(commands::currencies::CurrenciesArgs),
    /// Get a bridge quote (fees, estimated time, received amount)
    Quote(commands::quote::QuoteArgs),
    /// Execute a bridge transfer
    Bridge(commands::bridge::BridgeArgs),
    /// Check bridge transaction status
    Status(commands::status::StatusArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Chains(args) => commands::chains::run(args),
        Commands::Currencies(args) => commands::currencies::run(args),
        Commands::Quote(args) => commands::quote::run(args),
        Commands::Bridge(args) => commands::bridge::run(args).await,
        Commands::Status(args) => commands::status::run(args),
    }
}
