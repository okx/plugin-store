mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "spark-savings",
    about = "Spark Savings (sUSDS/sDAI) — earn Sky Savings Rate on your stablecoins",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Chain ID (default: 8453 Base)
    #[arg(long, global = true, default_value = "8453")]
    chain: u64,
    /// Wallet address override (defaults to active onchainos wallet)
    #[arg(long, global = true)]
    from: Option<String>,
    /// Simulate without broadcasting (dry run)
    #[arg(long, global = true, default_value = "false")]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current Sky Savings Rate (SSR/DSR) APY
    Apy {},
    /// Check your sUSDS (and sDAI on Ethereum) balance
    Balance {},
    /// Deposit USDS into sUSDS savings vault
    Deposit {
        /// Amount of USDS to deposit (e.g. 10.0)
        #[arg(long)]
        amount: f64,
    },
    /// Withdraw sUSDS back to USDS
    Withdraw {
        /// Amount of sUSDS shares to redeem (omit if using --all)
        #[arg(long)]
        amount: Option<f64>,
        /// Withdraw the full sUSDS balance
        #[arg(long, default_value = "false")]
        all: bool,
    },
    /// Show savings market info: TVL, rates, token addresses
    Markets {},
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let dry_run = cli.dry_run;
    let chain = cli.chain;
    let from = cli.from.as_deref();

    let result = match cli.command {
        Commands::Apy {} => commands::apy::run(chain).await,
        Commands::Balance {} => commands::balance::run(chain, from, dry_run).await,
        Commands::Deposit { amount } => commands::deposit::run(chain, amount, from, dry_run).await,
        Commands::Withdraw { amount, all } => {
            commands::withdraw::run(chain, amount, all, from, dry_run).await
        }
        Commands::Markets {} => commands::markets::run(chain).await,
    };

    match result {
        Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default()),
        Err(e) => {
            let err = serde_json::json!({
                "ok": false,
                "error": e.to_string()
            });
            eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
            std::process::exit(1);
        }
    }
}
