use clap::{Parser, Subcommand};

mod abi;
mod chain;
mod contracts;
mod nft;
mod onchainos;
mod token;
mod vault;
mod commands;

#[derive(Parser)]
#[command(
    name = "fluid-plugin",
    version = "0.1.1",
    about = "Fluid Protocol — lend, borrow, and manage positions on Ethereum and Arbitrum"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available vaults (default: T1 only; use --all for smart vaults)
    Vaults(commands::vaults::VaultsArgs),
    /// Show open positions for a wallet
    Positions(commands::positions::PositionsArgs),
    /// Supply collateral into a vault (opens new position if no --nft-id)
    Supply(commands::supply::SupplyArgs),
    /// Borrow debt token from an existing position
    Borrow(commands::borrow::BorrowArgs),
    /// Repay debt on an existing position
    Repay(commands::repay::RepayArgs),
    /// Withdraw collateral from an existing position
    Withdraw(commands::withdraw::WithdrawArgs),
    /// Close a position atomically: repay all debt and withdraw all collateral in one tx
    Close(commands::close::CloseArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Vaults(args)    => commands::vaults::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Supply(args)    => commands::supply::run(args).await,
        Commands::Borrow(args)    => commands::borrow::run(args).await,
        Commands::Repay(args)     => commands::repay::run(args).await,
        Commands::Withdraw(args)  => commands::withdraw::run(args).await,
        Commands::Close(args)     => commands::close::run(args).await,
    };
    if let Err(e) = result {
        eprintln!("[fluid] Error: {}", e);
        std::process::exit(1);
    }
}
