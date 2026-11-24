use clap::{Parser, Subcommand};

mod commands;
mod config;
mod onchainos;
mod rpc;

#[derive(Parser)]
#[command(name = "compound-v2-plugin", version, about = "Compound V2 (Ethereum mainnet) — exit tool for legacy cToken positions; supply/borrow paused, use compound-v3-plugin for active flows", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// First-time onboarding: scan ETH gas + 6 cToken markets for supply/borrow/COMP, return status enum + ready-to-run next_command
    Quickstart(commands::quickstart::QuickstartArgs),
    /// List markets — APYs, TVL, utilization, pause flags
    Markets(commands::markets::MarketsArgs),
    /// Show wallet's open positions across cToken markets (supply + borrow + accrued COMP)
    Positions(commands::positions::PositionsArgs),
    /// Supply underlying token to a cToken (BLOCKED in v0.1.0: all 6 markets paused — error redirects to compound-v3-plugin)
    Supply(commands::supply::SupplyArgs),
    /// Withdraw supplied underlying via cToken.redeemUnderlying (requires --confirm)
    Withdraw(commands::withdraw::WithdrawArgs),
    /// Borrow underlying token (requires --confirm; auto enterMarkets if needed)
    Borrow(commands::borrow::BorrowArgs),
    /// Repay debt (--all uses uint256.max sentinel for dust-free LEND-001 settle; requires --confirm)
    Repay(commands::repay::RepayArgs),
    /// Claim accumulated COMP rewards via Comptroller.claimComp (requires --confirm)
    ClaimComp(commands::claim_comp::ClaimCompArgs),
    /// Mark cTokens as collateral via Comptroller.enterMarkets (rarely needed; borrow auto-enters)
    EnterMarkets(commands::enter_markets::EnterMarketsArgs),
    /// Remove cToken from collateral set via Comptroller.exitMarket
    ExitMarket(commands::exit_market::ExitMarketArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quickstart(a)    => commands::quickstart::run(a).await,
        Commands::Markets(a)       => commands::markets::run(a).await,
        Commands::Positions(a)     => commands::positions::run(a).await,
        Commands::Supply(a)        => commands::supply::run(a).await,
        Commands::Withdraw(a)      => commands::withdraw::run(a).await,
        Commands::Borrow(a)        => commands::borrow::run(a).await,
        Commands::Repay(a)         => commands::repay::run(a).await,
        Commands::ClaimComp(a)     => commands::claim_comp::run(a).await,
        Commands::EnterMarkets(a)  => commands::enter_markets::run(a).await,
        Commands::ExitMarket(a)    => commands::exit_market::run(a).await,
    }
}
