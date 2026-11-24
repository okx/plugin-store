mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    balance::BalanceArgs,
    bridge::BridgeArgs,
    chains::ChainsArgs,
    quickstart::QuickstartArgs,
    quote::QuoteArgs,
    routes::RoutesArgs,
    status::StatusArgs,
    tokens::TokensArgs,
};

#[derive(Parser)]
#[command(
    name = "lifi-plugin",
    version,
    about = "LI.FI cross-chain bridge & swap aggregator (Ethereum, Arbitrum, Base, Optimism, BSC, Polygon)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// First-time onboarding: scan all 6 chains for funds and recommend a next step
    Quickstart(QuickstartArgs),
    /// List supported chains (use --all for the full LI.FI registry)
    Chains(ChainsArgs),
    /// List tokens on a chain (or look up one symbol)
    Tokens(TokensArgs),
    /// Get a single executable quote (calldata + price + fees)
    Quote(QuoteArgs),
    /// Plan multi-hop alternatives — returns N ranked routes (read-only)
    Routes(RoutesArgs),
    /// Execute a bridge / swap (requires --confirm)
    Bridge(BridgeArgs),
    /// Track an in-flight cross-chain transaction
    Status(StatusArgs),
    /// Read native + (optionally) one ERC-20 balance per chain
    Balance(BalanceArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quickstart(args) => commands::quickstart::run(args).await,
        Commands::Chains(args)     => commands::chains::run(args).await,
        Commands::Tokens(args)     => commands::tokens::run(args).await,
        Commands::Quote(args)      => commands::quote::run(args).await,
        Commands::Routes(args)     => commands::routes::run(args).await,
        Commands::Bridge(args)     => commands::bridge::run(args).await,
        Commands::Status(args)     => commands::status::run(args).await,
        Commands::Balance(args)    => commands::balance::run(args).await,
    }
}
