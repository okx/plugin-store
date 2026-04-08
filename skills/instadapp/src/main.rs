mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "instadapp",
    version = "0.1.0",
    about = "Instadapp Lite Vaults CLI — deposit ETH, withdraw, and track yield on Ethereum"
)]
struct Cli {
    /// Chain ID (default: 1 = Ethereum mainnet; Lite vaults are Ethereum-only)
    #[arg(long, default_value = "1")]
    chain: u64,

    /// Simulate without broadcasting on-chain transactions
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List Instadapp Lite vaults (iETH v1 and iETHv2) with exchange price and TVL
    Vaults,

    /// Show exchange price and cumulative yield for Instadapp Lite vaults
    Rates,

    /// Query your Instadapp Lite vault holdings (iETH and iETHv2 shares)
    Positions {
        /// Wallet address to query (default: resolve from onchainos)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Deposit ETH into Instadapp Lite iETH vault (v1) or stETH into iETHv2 (v2)
    Deposit {
        /// Vault to deposit into: "v1"/"iETH" for ETH vault, "v2"/"iETHv2" for stETH vault (default: v1)
        #[arg(long, default_value = "v1")]
        vault: String,

        /// Amount to deposit (ETH for v1, stETH for v2, e.g. "0.0001")
        #[arg(long)]
        amount: String,

        /// Wallet address (default: resolve from onchainos)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Withdraw from Instadapp Lite vault (burn iETH/iETHv2 shares to receive ETH/stETH)
    Withdraw {
        /// Vault to withdraw from: "v1"/"iETH" or "v2"/"iETHv2" (default: v1)
        #[arg(long, default_value = "v1")]
        vault: String,

        /// Number of shares to redeem (omit to redeem all)
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

    let result = match cli.command {
        Commands::Vaults => commands::vaults::execute(cli.chain).await,
        Commands::Rates => commands::rates::execute(cli.chain).await,
        Commands::Positions { wallet } => {
            commands::positions::execute(cli.chain, wallet.as_deref()).await
        }
        Commands::Deposit {
            vault,
            amount,
            wallet,
        } => {
            commands::deposit::execute(
                cli.chain,
                Some(&vault),
                &amount,
                cli.dry_run,
                wallet.as_deref(),
            )
            .await
        }
        Commands::Withdraw {
            vault,
            shares,
            wallet,
        } => {
            commands::withdraw::execute(
                cli.chain,
                Some(&vault),
                shares.as_deref(),
                cli.dry_run,
                wallet.as_deref(),
            )
            .await
        }
    };

    if let Err(e) = result {
        eprintln!(
            "{}",
            serde_json::json!({
                "ok": false,
                "error": e.to_string()
            })
        );
        std::process::exit(1);
    }
}
