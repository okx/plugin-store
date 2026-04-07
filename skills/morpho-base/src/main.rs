mod api;
mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "morpho-base", version = "0.1.0", about = "Supply, borrow and earn yield on Morpho V1 on Base — permissionless lending on the Base network")]
struct Cli {
    /// Chain ID: only 8453 (Base) is supported
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
    /// Supply assets to a MetaMorpho vault (ERC-4626 deposit)
    Supply {
        /// MetaMorpho vault address
        #[arg(long)]
        vault: String,

        /// Token symbol (USDC, WETH, cbETH, cbBTC) or ERC-20 address
        #[arg(long)]
        asset: String,

        /// Human-readable amount (e.g. 1000 or 0.5)
        #[arg(long)]
        amount: String,
    },

    /// Withdraw from a MetaMorpho vault (ERC-4626)
    Withdraw {
        /// MetaMorpho vault address
        #[arg(long)]
        vault: String,

        /// Token symbol or ERC-20 address
        #[arg(long)]
        asset: String,

        /// Human-readable amount to withdraw (mutually exclusive with --all)
        #[arg(long)]
        amount: Option<String>,

        /// Withdraw entire balance
        #[arg(long)]
        all: bool,
    },

    /// Borrow from a Morpho Blue market
    Borrow {
        /// Market unique key (bytes32 hex, e.g. 0xabc...)
        #[arg(long)]
        market_id: String,

        /// Human-readable amount to borrow
        #[arg(long)]
        amount: String,
    },

    /// Repay Morpho Blue debt
    Repay {
        /// Market unique key (bytes32 hex)
        #[arg(long)]
        market_id: String,

        /// Human-readable amount to repay (mutually exclusive with --all)
        #[arg(long)]
        amount: Option<String>,

        /// Repay entire outstanding balance
        #[arg(long)]
        all: bool,
    },

    /// View user positions and health factors
    Positions,

    /// List Morpho Blue markets with APYs
    Markets {
        /// Filter by loan asset symbol (e.g. USDC)
        #[arg(long)]
        asset: Option<String>,
    },

    /// Supply collateral to a Morpho Blue market
    SupplyCollateral {
        /// Market unique key (bytes32 hex)
        #[arg(long)]
        market_id: String,

        /// Human-readable amount of collateral to supply
        #[arg(long)]
        amount: String,
    },

    /// Claim Merkl rewards
    ClaimRewards,

    /// List MetaMorpho vaults with APYs
    Vaults {
        /// Filter by asset symbol (e.g. USDC)
        #[arg(long)]
        asset: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let chain_id = cli.chain;
    let dry_run = cli.dry_run;
    let from = cli.from.as_deref();

    let result = match cli.command {
        Commands::Supply { vault, asset, amount } => {
            commands::supply::run(&vault, &asset, &amount, chain_id, from, dry_run).await
        }
        Commands::Withdraw { vault, asset, amount, all } => {
            commands::withdraw::run(&vault, &asset, amount.as_deref(), all, chain_id, from, dry_run).await
        }
        Commands::Borrow { market_id, amount } => {
            commands::borrow::run(&market_id, &amount, chain_id, from, dry_run).await
        }
        Commands::Repay { market_id, amount, all } => {
            commands::repay::run(&market_id, amount.as_deref(), all, chain_id, from, dry_run).await
        }
        Commands::Positions => {
            commands::positions::run(chain_id, from).await
        }
        Commands::Markets { asset } => {
            commands::markets::run(chain_id, asset.as_deref()).await
        }
        Commands::SupplyCollateral { market_id, amount } => {
            commands::supply_collateral::run(&market_id, &amount, chain_id, from, dry_run).await
        }
        Commands::ClaimRewards => {
            commands::claim_rewards::run(chain_id, from, dry_run).await
        }
        Commands::Vaults { asset } => {
            commands::vaults::run(chain_id, asset.as_deref()).await
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
