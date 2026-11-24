use clap::{Parser, Subcommand};

mod commands;
mod config;
mod onchainos;
mod rpc;

#[derive(Parser)]
#[command(name = "aave-v2-plugin", version,
    about = "Aave V2 lending and borrowing on Ethereum, Polygon, and Avalanche - supply, borrow with stable or variable rate, repay (uint256.max sentinel for dust-free), claim stkAAVE rewards, swap rate mode",
    long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// First-time onboarding: scan native gas + Aave V2 reserves on selected chain (runtime enumerated), return status enum + ready-to-run next_command
    Quickstart(commands::quickstart::QuickstartArgs),
    /// List markets - APYs, TVL, utilization, LTV, liquidation threshold, frozen flag (runtime enumerated via getReservesList)
    Markets(commands::markets::MarketsArgs),
    /// Show wallet's open positions across reserves (supply + variable debt + stable debt + Health Factor + accrued rewards)
    Positions(commands::positions::PositionsArgs),
    /// Supply underlying token to Aave V2 (requires --confirm). Native ETH/MATIC/AVAX routes through WETHGateway
    Supply(commands::supply::SupplyArgs),
    /// Withdraw supplied underlying back to wallet (requires --confirm). Pass --amount all to redeem entire supply
    Withdraw(commands::withdraw::WithdrawArgs),
    /// Borrow underlying token (requires --confirm; pre-flight Health Factor check). --rate-mode 1=stable, 2=variable
    Borrow(commands::borrow::BorrowArgs),
    /// Repay debt (requires --confirm). --all uses uint256.max sentinel for dust-free LEND-001 settle
    Repay(commands::repay::RepayArgs),
    /// Claim accrued stkAAVE / WMATIC / WAVAX rewards from IncentivesController (requires --confirm)
    ClaimRewards(commands::claim_rewards::ClaimRewardsArgs),
    /// Swap an existing borrow between stable (1) and variable (2) interest rate mode (requires --confirm)
    SwapBorrowRateMode(commands::swap_borrow_rate_mode::SwapBorrowRateModeArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quickstart(a)            => commands::quickstart::run(a).await,
        Commands::Markets(a)               => commands::markets::run(a).await,
        Commands::Positions(a)             => commands::positions::run(a).await,
        Commands::Supply(a)                => commands::supply::run(a).await,
        Commands::Withdraw(a)              => commands::withdraw::run(a).await,
        Commands::Borrow(a)                => commands::borrow::run(a).await,
        Commands::Repay(a)                 => commands::repay::run(a).await,
        Commands::ClaimRewards(a)          => commands::claim_rewards::run(a).await,
        Commands::SwapBorrowRateMode(a)    => commands::swap_borrow_rate_mode::run(a).await,
    }
}
