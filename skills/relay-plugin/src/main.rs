mod api;
mod commands;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "relay-plugin", version, about = "Fast cross-chain transfers via Relay Protocol")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List supported chains
    Chains(commands::chains::ChainsArgs),
    /// Get a cross-chain transfer quote (read-only)
    Quote(commands::quote::QuoteArgs),
    /// Execute a cross-chain bridge transfer
    Bridge(commands::bridge::BridgeArgs),
    /// Check the status of a bridge transfer by request ID
    Status(commands::status::StatusArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Chains(args) => commands::chains::run(args).await,
        Commands::Quote(args)  => commands::quote::run(args).await,
        Commands::Bridge(args) => commands::bridge::run(args).await,
        Commands::Status(args) => commands::status::run(args).await,
    };
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
