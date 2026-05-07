mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "aave-v3",
    about = "Aave V3 lending and borrowing via OnchaionOS",
    version = env!("CARGO_PKG_VERSION")
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Chain ID (default: 8453 Base)
    #[arg(long, global = true, default_value = "8453")]
    chain: u64,
    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long, global = true)]
    from: Option<String>,
    /// Execute the transaction on-chain. Without this flag the operation is simulated only.
    #[arg(long, global = true, default_value = "false")]
    confirm: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Supply/deposit an asset to earn interest (aTokens)
    Supply {
        /// Asset ERC-20 address or symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: String,
        /// Human-readable amount (e.g. 1000.0)
        #[arg(long)]
        amount: f64,
    },
    /// Withdraw a previously supplied asset
    Withdraw {
        /// Asset ERC-20 address or symbol
        #[arg(long)]
        asset: String,
        /// Human-readable amount to withdraw (omit if using --all)
        #[arg(long)]
        amount: Option<f64>,
        /// Withdraw the full balance
        #[arg(long, default_value = "false")]
        all: bool,
    },
    /// Borrow an asset against posted collateral
    Borrow {
        /// Asset ERC-20 address or symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: String,
        /// Human-readable amount (e.g. 0.5 for 0.5 WETH)
        #[arg(long)]
        amount: f64,
    },
    /// Repay borrowed debt (partial or full)
    Repay {
        /// Asset ERC-20 address or symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: String,
        /// Human-readable amount to repay (omit if using --all)
        #[arg(long)]
        amount: Option<f64>,
        /// Repay the full outstanding balance
        #[arg(long, default_value = "false")]
        all: bool,
    },
    /// View current supply and borrow positions
    Positions {},
    /// Check health factor and liquidation risk
    HealthFactor {},
    /// List market rates, APYs, and liquidity for all assets
    Reserves {
        /// Filter by asset address or symbol (optional)
        #[arg(long)]
        asset: Option<String>,
    },
    /// Enable or disable an asset as collateral
    SetCollateral {
        /// Asset ERC-20 address or symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: String,
        /// true to enable as collateral, false to disable
        #[arg(long)]
        enable: bool,
    },
    /// Set efficiency mode (E-Mode) category
    SetEmode {
        /// E-Mode category ID: 0=none, 1=stablecoins, 2=ETH-correlated
        #[arg(long)]
        category: u8,
    },
    /// Claim accrued AAVE/GHO/token rewards
    ClaimRewards {},
    /// Check wallet assets and get a personalised next step for Aave V3
    Quickstart {},
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::Supply { asset, amount } => {
            commands::supply::run(cli.chain, &asset, amount, cli.from.as_deref(), !cli.confirm)
                .await
        }
        Commands::Withdraw { asset, amount, all } => {
            commands::withdraw::run(
                cli.chain,
                &asset,
                amount,
                all,
                cli.from.as_deref(),
                !cli.confirm,
            )
            .await
        }
        Commands::Borrow { asset, amount } => {
            commands::borrow::run(cli.chain, &asset, amount, cli.from.as_deref(), !cli.confirm)
                .await
        }
        Commands::Repay { asset, amount, all } => {
            commands::repay::run(
                cli.chain,
                &asset,
                amount,
                all,
                cli.from.as_deref(),
                !cli.confirm,
            )
            .await
        }
        Commands::Positions {} => {
            commands::positions::run(cli.chain, cli.from.as_deref()).await
        }
        Commands::HealthFactor {} => {
            commands::health_factor::run(cli.chain, cli.from.as_deref()).await
        }
        Commands::Reserves { asset } => {
            commands::reserves::run(cli.chain, asset.as_deref()).await
        }
        Commands::SetCollateral { asset, enable } => {
            commands::set_collateral::run(
                cli.chain,
                &asset,
                enable,
                cli.from.as_deref(),
                !cli.confirm,
            )
            .await
        }
        Commands::SetEmode { category } => {
            commands::set_emode::run(cli.chain, category, cli.from.as_deref(), !cli.confirm).await
        }
        Commands::ClaimRewards {} => {
            commands::claim_rewards::run(cli.chain, cli.from.as_deref(), !cli.confirm).await
        }
        Commands::Quickstart {} => {
            commands::quickstart::run(cli.chain, cli.from.as_deref()).await
        }
    };

    match result {
        Ok(val) => {
            println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        }
        Err(err) => {
            let error_json = serde_json::json!({
                "ok": false,
                "error": err.to_string()
            });
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&error_json).unwrap_or_default()
            );
            std::process::exit(1);
        }
    }
}
