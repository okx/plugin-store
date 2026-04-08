mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dolomite",
    version = "0.1.0",
    about = "Dolomite — Isolated lending markets on EVM chains (Arbitrum, Mantle, Berachain)"
)]
struct Cli {
    /// Chain ID: 42161 (Arbitrum), 5000 (Mantle), 80094 (Berachain)
    #[arg(long, default_value = "42161")]
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
    /// List Dolomite lending markets with TVL and interest rates
    Markets {
        /// Filter by asset symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: Option<String>,
    },

    /// View your Dolomite supply/borrow positions
    Positions,

    /// Supply tokens to Dolomite to earn lending yield
    Deposit {
        /// Asset symbol (USDC, WETH, USDT) or token address (0x...)
        #[arg(long)]
        asset: String,

        /// Human-readable amount to deposit (e.g. 10 or 0.001)
        #[arg(long)]
        amount: String,
    },

    /// Withdraw supplied tokens from Dolomite
    Withdraw {
        /// Asset symbol (USDC, WETH, USDT) or token address (0x...)
        #[arg(long)]
        asset: String,

        /// Human-readable amount to withdraw
        #[arg(long)]
        amount: Option<String>,

        /// Withdraw entire supplied balance
        #[arg(long)]
        all: bool,
    },

    /// Borrow tokens from Dolomite (dry-run only — liquidation risk)
    Borrow {
        /// Asset symbol (USDC, WETH, USDT) or token address (0x...)
        #[arg(long)]
        asset: String,

        /// Human-readable amount to borrow
        #[arg(long)]
        amount: String,
    },

    /// Repay borrowed tokens to Dolomite (dry-run only)
    Repay {
        /// Asset symbol (USDC, WETH, USDT) or token address (0x...)
        #[arg(long)]
        asset: String,

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
        Commands::Deposit { asset, amount } => {
            commands::deposit::run(&asset, &amount, chain_id, from, dry_run).await
        }
        Commands::Withdraw { asset, amount, all } => {
            commands::withdraw::run(&asset, amount.as_deref(), all, chain_id, from, dry_run).await
        }
        Commands::Borrow { asset, amount } => {
            commands::borrow::run(&asset, &amount, chain_id, from, dry_run).await
        }
        Commands::Repay { asset, amount, all } => {
            commands::repay::run(&asset, amount.as_deref(), all, chain_id, from, dry_run).await
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
