mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "swell-staking", about = "Swell Network liquid staking plugin for onchainos")]
struct Cli {
    /// Chain ID (only Ethereum mainnet = 1 is supported)
    #[arg(long, default_value_t = 1u64)]
    chain: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get current swETH and rswETH exchange rates
    Rates,
    /// Query swETH and rswETH positions for a wallet
    Positions(commands::positions::PositionsArgs),
    /// Stake ETH to receive swETH (liquid staking)
    Stake(commands::stake::StakeArgs),
    /// Restake ETH to receive rswETH (liquid restaking via EigenLayer)
    Restake(commands::restake::RestakeArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let chain = cli.chain;

    if chain != 1 {
        eprintln!("Warning: Swell staking only supports Ethereum mainnet (chain 1). Got chain {}.", chain);
    }

    match cli.command {
        Commands::Rates => commands::rates::run(chain).await,
        Commands::Positions(args) => commands::positions::run(args, chain).await,
        Commands::Stake(args) => commands::stake::run(args, chain).await,
        Commands::Restake(args) => commands::restake::run(args, chain).await,
    }
}
