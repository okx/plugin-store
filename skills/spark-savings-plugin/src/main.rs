mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    apy::ApyArgs,
    balance::BalanceArgs,
    deposit::DepositArgs,
    quickstart::QuickstartArgs,
    upgrade_dai::UpgradeDaiArgs,
    withdraw::WithdrawArgs,
};

#[derive(Parser)]
#[command(
    name = "spark-savings-plugin",
    version,
    about = "Spark Savings — earn Sky Savings Rate (SSR) on USDS via sUSDS yield-bearing vault on Ethereum, Base, and Arbitrum"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// First-time onboarding: scan USDS / sUSDS / DAI on all 3 chains and recommend a next step
    Quickstart(QuickstartArgs),
    /// Show live SSR (Sky Savings Rate), chi index, TVL — read from Ethereum mainnet (canonical)
    Apy(ApyArgs),
    /// Show USDS / sUSDS / DAI holdings + underlying USDS value of sUSDS shares
    Balance(BalanceArgs),
    /// Deposit USDS → sUSDS (ERC-4626 on Ethereum, Spark PSM on Base/Arbitrum)
    Deposit(DepositArgs),
    /// Redeem sUSDS → USDS (requires --confirm)
    Withdraw(WithdrawArgs),
    /// Upgrade legacy DAI → USDS 1:1 via the official DaiUsds migrator (Ethereum only)
    #[command(name = "upgrade-dai")]
    UpgradeDai(UpgradeDaiArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quickstart(args) => commands::quickstart::run(args).await,
        Commands::Apy(args)        => commands::apy::run(args).await,
        Commands::Balance(args)    => commands::balance::run(args).await,
        Commands::Deposit(args)    => commands::deposit::run(args).await,
        Commands::Withdraw(args)   => commands::withdraw::run(args).await,
        Commands::UpgradeDai(args) => commands::upgrade_dai::run(args).await,
    }
}
