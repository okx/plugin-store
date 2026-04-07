// Stader ETHx Liquid Staking Plugin
// Chain: Ethereum Mainnet (1)
// Commands: stake, unstake, claim, rates, positions

use clap::{Parser, Subcommand};

mod commands;
mod config;
mod onchainos;
mod rpc;

use commands::{claim, positions, rates, stake, unstake};

#[derive(Parser)]
#[command(name = "stader", about = "Stader ETHx liquid staking on Ethereum")]
struct Cli {
    /// Chain ID (default: 1 = Ethereum Mainnet)
    #[arg(long, default_value = "1")]
    chain: u64,

    /// Simulate without broadcasting to chain
    #[arg(long)]
    dry_run: bool,

    /// RPC URL override (default: https://ethereum.publicnode.com)
    #[arg(long, default_value = "https://ethereum.publicnode.com")]
    rpc_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deposit ETH to receive ETHx liquid staking token
    Stake(stake::StakeArgs),

    /// Request ETHx withdrawal (returns ETH after finalization, ~3-10 days)
    Unstake(unstake::UnstakeArgs),

    /// Claim finalized ETH withdrawal
    Claim(claim::ClaimArgs),

    /// Query current ETH <-> ETHx exchange rates and protocol stats
    Rates(rates::RatesArgs),

    /// Query your ETHx balance and pending withdrawal requests
    Positions(positions::PositionsArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Stake(args) => {
            stake::execute(&args, &cli.rpc_url, cli.chain, cli.dry_run).await
        }
        Commands::Unstake(args) => {
            unstake::execute(&args, &cli.rpc_url, cli.chain, cli.dry_run).await
        }
        Commands::Claim(args) => {
            claim::execute(&args, &cli.rpc_url, cli.chain, cli.dry_run).await
        }
        Commands::Rates(args) => {
            rates::execute(&args, &cli.rpc_url).await
        }
        Commands::Positions(args) => {
            positions::execute(&args, &cli.rpc_url, cli.chain, cli.dry_run).await
        }
    };

    if let Err(e) = result {
        let output = serde_json::json!({
            "ok": false,
            "error": e.to_string()
        });
        eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
        std::process::exit(1);
    }
}
