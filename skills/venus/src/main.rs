mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "venus", about = "Venus Core Pool - lending and borrowing on BSC")]
struct Cli {
    /// Chain ID (default: 56 BSC)
    #[arg(long, default_value = "56")]
    chain: u64,

    /// Simulate without broadcasting to chain
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all Venus Core Pool markets with APY and utilization
    GetMarkets,

    /// Show your current supply and borrow positions
    GetPositions {
        /// Wallet address (optional; defaults to logged-in wallet)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Supply (deposit) an asset to earn interest
    Supply {
        /// Asset symbol: BNB, USDT, BTC, ETH, USDC
        #[arg(long)]
        asset: String,
        /// Amount in human-readable units (e.g. 10.0 for 10 USDT)
        #[arg(long)]
        amount: f64,
    },

    /// Withdraw (redeem) a supplied asset
    Withdraw {
        /// Asset symbol: BNB, USDT, BTC, ETH, USDC
        #[arg(long)]
        asset: String,
        /// Amount of underlying to withdraw
        #[arg(long)]
        amount: f64,
    },

    /// Borrow an asset against your collateral
    Borrow {
        /// Asset symbol: BNB, USDT, BTC, ETH, USDC
        #[arg(long)]
        asset: String,
        /// Amount to borrow
        #[arg(long)]
        amount: f64,
    },

    /// Repay borrowed assets
    Repay {
        /// Asset symbol: BNB, USDT, BTC, ETH, USDC
        #[arg(long)]
        asset: String,
        /// Amount to repay
        #[arg(long)]
        amount: f64,
    },

    /// Enable an asset as collateral (enterMarkets)
    EnterMarket {
        /// Asset symbol: BNB, USDT, BTC, ETH, USDC
        #[arg(long)]
        asset: String,
    },

    /// Claim XVS rewards from Venus Comptroller
    ClaimRewards {
        /// Wallet address (optional; defaults to logged-in wallet)
        #[arg(long)]
        wallet: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::GetMarkets => commands::get_markets::execute(cli.chain).await,
        Commands::GetPositions { wallet } => {
            commands::get_positions::execute(cli.chain, wallet).await
        }
        Commands::Supply { asset, amount } => {
            commands::supply::execute(cli.chain, &asset, amount, cli.dry_run).await
        }
        Commands::Withdraw { asset, amount } => {
            commands::withdraw::execute(cli.chain, &asset, amount, cli.dry_run).await
        }
        Commands::Borrow { asset, amount } => {
            commands::borrow::execute(cli.chain, &asset, amount, cli.dry_run).await
        }
        Commands::Repay { asset, amount } => {
            commands::repay::execute(cli.chain, &asset, amount, cli.dry_run).await
        }
        Commands::EnterMarket { asset } => {
            commands::enter_market::execute(cli.chain, &asset, cli.dry_run).await
        }
        Commands::ClaimRewards { wallet } => {
            commands::claim_rewards::execute(cli.chain, wallet, cli.dry_run).await
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
