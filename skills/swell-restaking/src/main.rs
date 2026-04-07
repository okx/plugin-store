mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "swell-restaking", about = "Swell Restaking plugin: stake ETH to receive rswETH (EigenLayer liquid restaking) on Ethereum mainnet")]
struct Cli {
    /// Chain ID (only Ethereum mainnet = 1 is supported)
    #[arg(long, default_value_t = 1u64)]
    chain: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get current rswETH exchange rates and pool stats
    GetRates,
    /// Query rswETH balance and ETH-equivalent value for a wallet
    GetPositions(commands::get_positions::GetPositionsArgs),
    /// Stake ETH to receive rswETH (EigenLayer liquid restaking)
    Stake(commands::stake::StakeArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let chain = cli.chain;

    if chain != 1 {
        eprintln!("Warning: Swell Restaking only supports Ethereum mainnet (chain 1). Got chain {}.", chain);
    }

    match cli.command {
        Commands::GetRates => commands::get_rates::run(chain).await,
        Commands::GetPositions(args) => commands::get_positions::run(args, chain).await,
        Commands::Stake(args) => commands::stake::run(args, chain).await,
    }
}
