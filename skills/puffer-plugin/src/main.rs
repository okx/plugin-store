mod api;
mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    claim_withdraw::ClaimWithdrawArgs,
    instant_withdraw::InstantWithdrawArgs,
    positions::PositionsArgs,
    quickstart::QuickstartArgs,
    rate::RateArgs,
    request_withdraw::RequestWithdrawArgs,
    stake::StakeArgs,
    withdraw_options::WithdrawOptionsArgs,
    withdraw_status::WithdrawStatusArgs,
};

#[derive(Parser)]
#[command(
    name = "puffer",
    version,
    about = "Puffer Finance liquid restaking plugin — stake ETH for pufETH, compare 1-step vs 2-step withdraw paths, claim queued withdrawals"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// First-time onboarding: scan ETH + pufETH balance, current rate + APY, return status enum + ready-to-run next_command.
    Quickstart(QuickstartArgs),
    /// Show pufETH balance, ETH-equivalent value, current rate, exit fee, and APY (read-only).
    Positions(PositionsArgs),
    /// Deposit ETH into PufferVault to receive pufETH (ERC-4626 mint, 1:1 ≤ rate).
    Stake(StakeArgs),
    /// Show current pufETH↔ETH rate, exit-fee, and withdrawal parameters (read-only).
    Rate(RateArgs),
    /// Preview both withdraw paths (1-step instant with 1% fee vs 2-step queued ~14d no fee). Read-only.
    WithdrawOptions(WithdrawOptionsArgs),
    /// Start a 2-step queued withdrawal (fee-free, finalizes in ~14 days). Returns the withdrawal index.
    RequestWithdraw(RequestWithdrawArgs),
    /// Check status of a 2-step withdrawal by index. Tells the caller whether `claim-withdraw` is ready.
    WithdrawStatus(WithdrawStatusArgs),
    /// Claim a finalized 2-step withdrawal (step 2). Requires the withdrawal to be in a finalized batch.
    ClaimWithdraw(ClaimWithdrawArgs),
    /// 1-step instant withdraw: redeem pufETH → WETH in one tx, minus the exit fee (default 1%).
    InstantWithdraw(InstantWithdrawArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Quickstart(args) => commands::quickstart::run(args).await,
        Commands::Positions(args) => commands::positions::run(args).await,
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::Rate(args) => commands::rate::run(args).await,
        Commands::WithdrawOptions(args) => commands::withdraw_options::run(args).await,
        Commands::RequestWithdraw(args) => commands::request_withdraw::run(args).await,
        Commands::WithdrawStatus(args) => commands::withdraw_status::run(args).await,
        Commands::ClaimWithdraw(args) => commands::claim_withdraw::run(args).await,
        Commands::InstantWithdraw(args) => commands::instant_withdraw::run(args).await,
    }
}
