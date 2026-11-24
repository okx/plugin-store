mod api;
mod calldata;
mod commands;
mod config;
mod multicall;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "euler-v2-plugin",
    version,
    about = "Supply, borrow and earn yield on Euler v2 — modular lending protocol with isolated-risk EVK vaults"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // ── Read commands ────────────────────────────────────────────────────
    /// Check wallet status and get a guided next step (status routing, no auth required)
    Quickstart(commands::quickstart::QuickstartArgs),

    /// List EVK vaults on a chain (no auth required)
    ListVaults(commands::list_vaults::ListVaultsArgs),

    /// Get full details for a single EVK vault (no auth required)
    GetVault(commands::get_vault::GetVaultArgs),

    /// Show user's supply / borrow positions across all EVK vaults (read-only)
    Positions(commands::positions::PositionsArgs),

    /// Show user's liquidation buffer status (no_borrow / borrow_present / no_position)
    HealthFactor(commands::health_factor::HealthFactorArgs),

    // ── Write commands ───────────────────────────────────────────────────
    /// Deposit asset into an EVK vault (mint vault shares)
    Supply(commands::supply::SupplyArgs),

    /// Burn vault shares to retrieve underlying asset (use --all for full redeem)
    Withdraw(commands::withdraw::WithdrawArgs),

    /// Borrow underlying from a controller vault (run enable-controller first)
    Borrow(commands::borrow::BorrowArgs),

    /// Repay borrow position (use --all for full repay including accrued interest)
    Repay(commands::repay::RepayArgs),

    /// Designate a vault's shares as collateral via EVC
    EnableCollateral(commands::enable_collateral::EnableCollateralArgs),

    /// Un-designate a vault as collateral (only if it doesn't break account health)
    DisableCollateral(commands::disable_collateral::DisableCollateralArgs),

    /// Designate a vault as the borrower (required before borrow)
    EnableController(commands::enable_controller::EnableControllerArgs),

    /// Release the borrower-vault designation (required after full repay before withdrawing all collateral)
    DisableController(commands::disable_controller::DisableControllerArgs),

    /// Claim reward streams (Merkl / Brevis / Fuul) — v0.1 stub, planned for v0.2
    ClaimRewards(commands::claim_rewards::ClaimRewardsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quickstart(args)         => commands::quickstart::run(args).await,
        Commands::ListVaults(args)         => commands::list_vaults::run(args).await,
        Commands::GetVault(args)           => commands::get_vault::run(args).await,
        Commands::Positions(args)          => commands::positions::run(args).await,
        Commands::HealthFactor(args)       => commands::health_factor::run(args).await,
        Commands::Supply(args)             => commands::supply::run(args).await,
        Commands::Withdraw(args)           => commands::withdraw::run(args).await,
        Commands::Borrow(args)             => commands::borrow::run(args).await,
        Commands::Repay(args)              => commands::repay::run(args).await,
        Commands::EnableCollateral(args)   => commands::enable_collateral::run(args).await,
        Commands::DisableCollateral(args)  => commands::disable_collateral::run(args).await,
        Commands::EnableController(args)   => commands::enable_controller::run(args).await,
        Commands::DisableController(args)  => commands::disable_controller::run(args).await,
        Commands::ClaimRewards(args)       => commands::claim_rewards::run(args).await,
    }
}
