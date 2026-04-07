mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "compound-v2", about = "Compound V2 cToken lending plugin")]
struct Cli {
    /// Chain ID (1 = Ethereum mainnet)
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
    /// List cToken markets with supply/borrow APR and exchange rates
    Markets,

    /// View your supplied and borrowed positions across all markets
    Positions {
        /// Wallet address (defaults to logged-in onchainos wallet)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Supply an asset to earn interest (mints cTokens)
    Supply {
        /// Asset symbol: ETH, USDT, USDC, DAI
        #[arg(long)]
        asset: String,

        /// Human-readable amount (e.g. 0.01 for 0.01 USDT)
        #[arg(long)]
        amount: f64,

        /// Sender wallet (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Redeem cTokens to get back underlying asset
    Redeem {
        /// Asset symbol: ETH, USDT, USDC, DAI
        #[arg(long)]
        asset: String,

        /// cToken amount to redeem (in cToken units, 8 decimals)
        #[arg(long)]
        ctoken_amount: f64,

        /// Sender wallet (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Borrow an asset (DRY-RUN ONLY — requires collateral)
    Borrow {
        /// Asset symbol: ETH, USDT, USDC, DAI
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
        /// Asset symbol: ETH, USDT, USDC, DAI
        #[arg(long)]
        asset: String,

        /// Human-readable repay amount
        #[arg(long)]
        amount: f64,

        /// Sender wallet (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Claim accrued COMP rewards from the Comptroller
    ClaimComp {
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
        Commands::Redeem { asset, ctoken_amount, from } => {
            commands::redeem::run(cli.chain, asset, ctoken_amount, from, cli.dry_run).await
        }
        Commands::Borrow { asset, amount, from } => {
            commands::borrow::run(cli.chain, asset, amount, from, cli.dry_run).await
        }
        Commands::Repay { asset, amount, from } => {
            commands::repay::run(cli.chain, asset, amount, from, cli.dry_run).await
        }
        Commands::ClaimComp { from } => {
            commands::claim_comp::run(cli.chain, from, cli.dry_run).await
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
