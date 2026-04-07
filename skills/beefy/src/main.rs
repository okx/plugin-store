mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "beefy",
    version = "0.1.0",
    about = "Beefy Finance yield optimizer CLI - deposit into auto-compounding vaults"
)]
struct Cli {
    /// Chain ID (e.g. 8453=Base, 56=BSC, 1=Ethereum)
    #[arg(long, global = true, default_value = "8453")]
    chain: u64,

    /// Simulate without broadcasting on-chain transactions
    #[arg(long, global = true, default_value = "false")]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List Beefy vaults with APY and TVL
    Vaults {
        /// Filter by asset symbol (e.g. "USDC", "WETH")
        #[arg(long)]
        asset: Option<String>,
        /// Filter by platform (e.g. "aerodrome", "morpho")
        #[arg(long)]
        platform: Option<String>,
        /// Maximum number of vaults to show (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Show APY data for Beefy vaults
    Apy {
        /// Specific vault ID to query
        #[arg(long)]
        vault: Option<String>,
        /// Filter by asset symbol
        #[arg(long)]
        asset: Option<String>,
        /// Maximum results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// View your Beefy vault positions (mooToken balances)
    Positions {
        /// Wallet address (default: resolve from onchainos)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Deposit tokens into a Beefy vault (auto-compounding)
    Deposit {
        /// Vault ID or earn contract address (e.g. "morpho-base-gauntlet-prime-usdc")
        #[arg(long)]
        vault: String,
        /// Amount to deposit in human-readable form (e.g. "0.01" for 0.01 USDC)
        #[arg(long)]
        amount: String,
        /// Wallet address (default: resolve from onchainos)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Withdraw tokens from a Beefy vault (redeem mooTokens)
    Withdraw {
        /// Vault ID or earn contract address
        #[arg(long)]
        vault: String,
        /// Number of mooToken shares to redeem (omit to redeem all)
        #[arg(long)]
        shares: Option<String>,
        /// Wallet address (default: resolve from onchainos)
        #[arg(long)]
        wallet: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::Vaults { asset, platform, limit } => {
            commands::vaults::execute(cli.chain, asset.as_deref(), platform.as_deref(), limit).await
        }
        Commands::Apy { vault, asset, limit } => {
            commands::apy::execute(cli.chain, vault.as_deref(), asset.as_deref(), limit).await
        }
        Commands::Positions { wallet } => {
            commands::positions::execute(cli.chain, wallet.as_deref()).await
        }
        Commands::Deposit { vault, amount, wallet } => {
            commands::deposit::execute(
                cli.chain,
                &vault,
                &amount,
                cli.dry_run,
                wallet.as_deref(),
            )
            .await
        }
        Commands::Withdraw { vault, shares, wallet } => {
            commands::withdraw::execute(
                cli.chain,
                &vault,
                shares.as_deref(),
                cli.dry_run,
                wallet.as_deref(),
            )
            .await
        }
    };

    match result {
        Ok(val) => {
            println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        }
        Err(e) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": false,
                    "error": e.to_string()
                }))
                .unwrap_or_default()
            );
            std::process::exit(1);
        }
    }
}
