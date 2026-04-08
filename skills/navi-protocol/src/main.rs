mod api;
mod commands;
mod config;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "navi-protocol",
    version = "0.1.0",
    about = "NAVI Protocol CLI — lend and borrow on Sui's leading lending protocol",
    long_about = "\
NAVI Protocol CLI

Interact with NAVI Protocol, the leading lending/borrowing protocol on Sui blockchain.

READ COMMANDS (live Sui mainnet data):
  reserves   — list all lending markets with APY and utilization
  positions  — show a wallet's supply/borrow balances and health factor

WRITE COMMANDS (preview only — Sui write ops not yet supported by onchainos):
  supply     — preview a supply/deposit transaction
  withdraw   — preview a withdrawal transaction
  borrow     — preview a borrow transaction
  repay      — preview a repay transaction

All write commands show the Move call that would be submitted. Add --confirm to
see the error message explaining how to execute via NAVI app or TypeScript SDK.

Chain: Sui Mainnet (chain ID 784)
RPC:   https://fullnode.mainnet.sui.io
API:   https://open-api.naviprotocol.io"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all NAVI lending reserves with APY, utilization, and pool info
    Reserves(commands::reserves::ReservesArgs),

    /// Show user positions: supply/borrow balances and health factor
    Positions(commands::positions::PositionsArgs),

    /// Preview supplying an asset to NAVI (write preview only)
    Supply(commands::supply::SupplyArgs),

    /// Preview withdrawing an asset from NAVI (write preview only)
    Withdraw(commands::withdraw::WithdrawArgs),

    /// Preview borrowing an asset from NAVI (write preview only)
    Borrow(commands::borrow::BorrowArgs),

    /// Preview repaying a borrow on NAVI (write preview only)
    Repay(commands::repay::RepayArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Reserves(args) => commands::reserves::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Supply(args) => commands::supply::run(args).await,
        Commands::Withdraw(args) => commands::withdraw::run(args).await,
        Commands::Borrow(args) => commands::borrow::run(args).await,
        Commands::Repay(args) => commands::repay::run(args).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
