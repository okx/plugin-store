mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "euler-v2",
    version = "0.1.0",
    about = "Euler V2 — Modular ERC-4626 lending vaults (EVaults) on EVM chains"
)]
struct Cli {
    /// Chain ID: 1 (Ethereum), 8453 (Base), 42161 (Arbitrum), 43114 (Avalanche), 56 (BSC)
    #[arg(long, default_value = "8453")]
    chain: u64,

    /// Simulate without broadcasting on-chain
    #[arg(long)]
    dry_run: bool,

    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long)]
    from: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List Euler V2 lending markets (EVaults) with TVL and rates
    Markets {
        /// Filter by asset symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: Option<String>,
    },

    /// View your Euler V2 supply/borrow positions
    Positions,

    /// Supply underlying assets into an EVault (ERC-4626 deposit)
    Supply {
        /// EVault address (0x...) or asset symbol (USDC, WETH, CBBTC)
        #[arg(long)]
        vault: String,

        /// Human-readable amount to supply (e.g. 10 or 0.001)
        #[arg(long)]
        amount: String,
    },

    /// Withdraw underlying assets from an EVault
    Withdraw {
        /// EVault address (0x...) or asset symbol (USDC, WETH)
        #[arg(long)]
        vault: String,

        /// Human-readable amount to withdraw
        #[arg(long)]
        amount: Option<String>,

        /// Withdraw entire balance (redeem all shares)
        #[arg(long)]
        all: bool,
    },

    /// Borrow assets from an EVault (dry-run only — liquidation risk)
    Borrow {
        /// EVault address (0x...) or asset symbol
        #[arg(long)]
        vault: String,

        /// Human-readable amount to borrow
        #[arg(long)]
        amount: String,
    },

    /// Repay borrowed assets to an EVault (dry-run only)
    Repay {
        /// EVault address (0x...) or asset symbol
        #[arg(long)]
        vault: String,

        /// Human-readable amount to repay
        #[arg(long)]
        amount: Option<String>,

        /// Repay all outstanding debt
        #[arg(long)]
        all: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let chain_id = cli.chain;
    let dry_run = cli.dry_run;
    let from = cli.from.as_deref();

    let result = match cli.command {
        Commands::Markets { asset } => {
            commands::markets::run(chain_id, asset.as_deref()).await
        }
        Commands::Positions => {
            commands::positions::run(chain_id, from, dry_run).await
        }
        Commands::Supply { vault, amount } => {
            commands::supply::run(&vault, &amount, chain_id, from, dry_run).await
        }
        Commands::Withdraw { vault, amount, all } => {
            commands::withdraw::run(&vault, amount.as_deref(), all, chain_id, from, dry_run).await
        }
        Commands::Borrow { vault, amount } => {
            commands::borrow::run(&vault, &amount, chain_id, from, dry_run).await
        }
        Commands::Repay { vault, amount, all } => {
            commands::repay::run(&vault, amount.as_deref(), all, chain_id, from, dry_run).await
        }
    };

    if let Err(e) = result {
        let err_out = serde_json::json!({
            "ok": false,
            "error": e.to_string(),
        });
        eprintln!("{}", serde_json::to_string_pretty(&err_out).unwrap_or_else(|_| e.to_string()));
        std::process::exit(1);
    }
}
