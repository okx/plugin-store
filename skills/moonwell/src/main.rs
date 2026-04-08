mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "moonwell", about = "Moonwell Flagship lending/borrowing plugin (Compound V2 fork)")]
struct Cli {
    /// Chain ID (8453 = Base, 10 = Optimism, 1284 = Moonbeam)
    #[arg(long, default_value = "8453")]
    chain: u64,

    /// Simulate without broadcasting on-chain transactions
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List mToken markets with supply/borrow APR and exchange rates
    Markets,

    /// View your supplied and borrowed positions across all markets
    Positions {
        /// Wallet address (defaults to logged-in onchainos wallet)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Supply an asset to earn interest (mints mTokens)
    Supply {
        /// Asset symbol: USDC, WETH, cbETH, USDbC, DAI
        #[arg(long)]
        asset: String,

        /// Human-readable amount (e.g. 0.01 for 0.01 USDC)
        #[arg(long)]
        amount: f64,

        /// Sender wallet (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Redeem mTokens to get back underlying asset
    Redeem {
        /// Asset symbol: USDC, WETH, cbETH, USDbC, DAI
        #[arg(long)]
        asset: String,

        /// mToken amount to redeem (in mToken units, 8 decimals)
        #[arg(long)]
        mtoken_amount: f64,

        /// Sender wallet (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Borrow an asset (DRY-RUN ONLY — requires collateral, liquidation risk)
    Borrow {
        /// Asset symbol: USDC, WETH, cbETH, USDbC, DAI
        #[arg(long)]
        asset: String,

        /// Human-readable borrow amount
        #[arg(long)]
        amount: f64,

        /// Sender wallet (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Repay a borrow (DRY-RUN ONLY)
    Repay {
        /// Asset symbol: USDC, WETH, cbETH, USDbC, DAI
        #[arg(long)]
        asset: String,

        /// Human-readable repay amount
        #[arg(long)]
        amount: f64,

        /// Sender wallet (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Claim accrued WELL rewards from the Comptroller
    ClaimRewards {
        /// Sender wallet (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Markets => {
            commands::markets::run(cli.chain).await
        }
        Commands::Positions { wallet } => {
            commands::positions::run(cli.chain, wallet).await
        }
        Commands::Supply { asset, amount, from } => {
            commands::supply::run(cli.chain, asset, amount, from, cli.dry_run).await
        }
        Commands::Redeem { asset, mtoken_amount, from } => {
            commands::redeem::run(cli.chain, asset, mtoken_amount, from, cli.dry_run).await
        }
        Commands::Borrow { asset, amount, from } => {
            commands::borrow::run(cli.chain, asset, amount, from, cli.dry_run).await
        }
        Commands::Repay { asset, amount, from } => {
            commands::repay::run(cli.chain, asset, amount, from, cli.dry_run).await
        }
        Commands::ClaimRewards { from } => {
            commands::claim_rewards::run(cli.chain, from, cli.dry_run).await
        }
    };

    match result {
        Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap()),
        Err(e) => {
            let err = serde_json::json!({"ok": false, "error": e.to_string()});
            eprintln!("{}", serde_json::to_string_pretty(&err).unwrap());
            std::process::exit(1);
        }
    }
}
