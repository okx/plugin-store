mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "renzo", about = "Renzo EigenLayer restaking plugin for onchainos")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deposit native ETH to receive ezETH (liquid restaking token)
    DepositEth(commands::deposit_eth::DepositEthArgs),
    /// Deposit stETH to receive ezETH (requires approve + deposit)
    DepositSteth(commands::deposit_steth::DepositStethArgs),
    /// Get current Renzo restaking APR
    GetApr,
    /// Check ezETH and stETH balances for an address
    Balance(commands::balance::BalanceArgs),
    /// Get Renzo protocol TVL
    GetTvl,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::DepositEth(args) => commands::deposit_eth::run(args).await,
        Commands::DepositSteth(args) => commands::deposit_steth::run(args).await,
        Commands::GetApr => commands::get_apr::run().await,
        Commands::Balance(args) => commands::balance::run(args).await,
        Commands::GetTvl => commands::get_tvl::run().await,
    }
}
