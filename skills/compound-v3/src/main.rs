mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "compound-v3", about = "Compound V3 (Comet) lending plugin")]
struct Cli {
    /// Chain ID (1=Ethereum, 8453=Base, 42161=Arbitrum, 137=Polygon)
    #[arg(long, default_value = "8453")]
    chain: u64,

    /// Market name (usdc, weth, usdt)
    #[arg(long, default_value = "usdc")]
    market: String,

    /// Simulate without broadcasting on-chain transactions
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List market info: supply APR, borrow APR, utilization, TVL
    GetMarkets,

    /// View account position: supply balance, borrow balance, collateral
    GetPosition {
        /// Wallet address (defaults to logged-in onchainos wallet)
        #[arg(long)]
        wallet: Option<String>,

        /// Collateral asset address to check collateral balance for
        #[arg(long)]
        collateral_asset: Option<String>,
    },

    /// Supply collateral or base asset (also used for repaying debt)
    Supply {
        /// Token contract address to supply
        #[arg(long)]
        asset: String,

        /// Amount in token's minimal units (e.g. 1000000 = 1 USDC)
        #[arg(long)]
        amount: u128,

        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Borrow base asset (implemented via Comet.withdraw)
    Borrow {
        /// Amount of base asset to borrow in minimal units (e.g. 1000000 = 1 USDC)
        #[arg(long)]
        amount: u128,

        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Repay borrowed base asset
    Repay {
        /// Amount to repay in minimal units. Omit to repay all debt.
        #[arg(long)]
        amount: Option<u128>,

        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Withdraw supplied collateral (requires zero borrow balance)
    Withdraw {
        /// Token contract address to withdraw
        #[arg(long)]
        asset: String,

        /// Amount in token's minimal units
        #[arg(long)]
        amount: u128,

        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Claim COMP rewards from the CometRewards contract
    ClaimRewards {
        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::GetMarkets => {
            commands::get_markets::run(cli.chain, &cli.market).await
        }
        Commands::GetPosition { wallet, collateral_asset } => {
            commands::get_position::run(cli.chain, &cli.market, wallet, collateral_asset).await
        }
        Commands::Supply { asset, amount, from } => {
            commands::supply::run(cli.chain, &cli.market, &asset, amount, from, cli.dry_run).await
        }
        Commands::Borrow { amount, from } => {
            commands::borrow::run(cli.chain, &cli.market, amount, from, cli.dry_run).await
        }
        Commands::Repay { amount, from } => {
            commands::repay::run(cli.chain, &cli.market, amount, from, cli.dry_run).await
        }
        Commands::Withdraw { asset, amount, from } => {
            commands::withdraw::run(cli.chain, &cli.market, &asset, amount, from, cli.dry_run).await
        }
        Commands::ClaimRewards { from } => {
            commands::claim_rewards::run(cli.chain, &cli.market, from, cli.dry_run).await
        }
    };

    if let Err(e) = result {
        let err_output = serde_json::json!({
            "ok": false,
            "error": e.to_string()
        });
        eprintln!("{}", serde_json::to_string_pretty(&err_output).unwrap());
        std::process::exit(1);
    }
}
