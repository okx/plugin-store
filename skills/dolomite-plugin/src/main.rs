mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    borrow::BorrowArgs,
    markets::MarketsArgs,
    positions::PositionsArgs,
    quickstart::QuickstartArgs,
    repay::RepayArgs,
    supply::SupplyArgs,
    withdraw::WithdrawArgs,
};

#[derive(Parser)]
#[command(
    name = "dolomite-plugin",
    version,
    about = "Dolomite Finance lending/borrowing on Arbitrum — supply assets, open isolated borrow positions, repay, withdraw via DolomiteMargin"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// First-time onboarding: scan ETH + 7 most-common markets, return status enum + ready-to-run next_command
    Quickstart(QuickstartArgs),
    /// List markets + supply/borrow APYs + utilization (read-only). Use --all for full enumeration.
    Markets(MarketsArgs),
    /// Show wallet's open positions across markets (supply + borrow + USD-equivalent values)
    Positions(PositionsArgs),
    /// Supply a token to earn interest (deposit into DolomiteMargin via DepositWithdrawalProxy)
    Supply(SupplyArgs),
    /// Withdraw a previously-supplied token (requires --confirm)
    Withdraw(WithdrawArgs),
    /// Borrow a token against existing collateral (requires --confirm; under-collateralized → revert)
    Borrow(BorrowArgs),
    /// Repay debt for a token; pass --all to clear the entire borrow (requires --confirm)
    Repay(RepayArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quickstart(args) => commands::quickstart::run(args).await,
        Commands::Markets(args)    => commands::markets::run(args).await,
        Commands::Positions(args)  => commands::positions::run(args).await,
        Commands::Supply(args)     => commands::supply::run(args).await,
        Commands::Withdraw(args)   => commands::withdraw::run(args).await,
        Commands::Borrow(args)     => commands::borrow::run(args).await,
        Commands::Repay(args)      => commands::repay::run(args).await,
    }
}
