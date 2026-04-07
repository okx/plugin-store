mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "notional-v3", about = "Notional Finance leveraged yield (Exponent) plugin")]
struct Cli {
    /// Chain ID (only Ethereum mainnet=1 is supported)
    #[arg(long, default_value = "1")]
    chain: u64,

    /// Dry-run mode: print calldata without sending transactions
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List available leveraged yield vaults
    GetVaults(commands::get_vaults::GetVaultsArgs),
    /// Get current vault positions for a wallet
    GetPositions(commands::get_positions::GetPositionsArgs),
    /// Enter a leveraged yield position (deposit + optional borrow)
    EnterPosition(commands::enter_position::EnterPositionArgs),
    /// Exit a vault position by redeeming shares
    ExitPosition(commands::exit_position::ExitPositionArgs),
    /// Initiate withdrawal (for staking strategies with unstaking period)
    InitiateWithdraw(commands::initiate_withdraw::InitiateWithdrawArgs),
    /// Claim pending rewards from a vault
    ClaimRewards(commands::claim_rewards::ClaimRewardsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.chain != config::ETHEREUM_CHAIN_ID {
        anyhow::bail!(
            "Chain {} is not supported. Notional Exponent is only deployed on Ethereum mainnet (chain 1).",
            cli.chain
        );
    }

    match &cli.command {
        Commands::GetVaults(args) => {
            commands::get_vaults::execute(args).await?;
        }
        Commands::GetPositions(args) => {
            commands::get_positions::execute(args, cli.dry_run).await?;
        }
        Commands::EnterPosition(args) => {
            commands::enter_position::execute(args, cli.dry_run, cli.chain).await?;
        }
        Commands::ExitPosition(args) => {
            commands::exit_position::execute(args, cli.dry_run, cli.chain).await?;
        }
        Commands::InitiateWithdraw(args) => {
            commands::initiate_withdraw::execute(args, cli.dry_run, cli.chain).await?;
        }
        Commands::ClaimRewards(args) => {
            commands::claim_rewards::execute(args, cli.dry_run, cli.chain).await?;
        }
    }

    Ok(())
}
