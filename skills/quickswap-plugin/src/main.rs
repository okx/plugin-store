mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "quickswap-plugin",
    version = env!("CARGO_PKG_VERSION"),
    about = "QuickSwap V3 DEX on Polygon — swap tokens via Algebra Protocol CLMM"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Broadcast on-chain (omit for dry-run preview)
    #[arg(long, global = true)]
    confirm: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Swap tokens on QuickSwap V3 (Algebra Protocol CLMM)
    Swap {
        /// Input token symbol or address (e.g. MATIC, USDC, 0x...)
        #[arg(long)]
        token_in: String,
        /// Output token symbol or address (e.g. USDC, WETH, 0x...)
        #[arg(long)]
        token_out: String,
        /// Amount of tokenIn to swap (human-readable, e.g. 10.5)
        #[arg(long)]
        amount: f64,
        /// Maximum slippage tolerance in percent (default: 0.5)
        #[arg(long, default_value = "0.5")]
        slippage: f64,
        /// Override sender address (defaults to onchainos wallet)
        #[arg(long)]
        from: Option<String>,
    },
    /// Get a swap quote without executing a transaction
    Quote {
        /// Input token symbol or address
        #[arg(long)]
        token_in: String,
        /// Output token symbol or address
        #[arg(long)]
        token_out: String,
        /// Amount of tokenIn (human-readable)
        #[arg(long)]
        amount: f64,
    },
    /// List top QuickSwap V3 pools by TVL
    Pools {
        /// Number of pools to return (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let dry_run = !cli.confirm;

    let result = match cli.command {
        Commands::Swap {
            token_in,
            token_out,
            amount,
            slippage,
            from,
        } => {
            commands::swap::run(
                config::CHAIN_ID,
                &token_in,
                &token_out,
                amount,
                slippage,
                from.as_deref(),
                dry_run,
            )
            .await
        }
        Commands::Quote {
            token_in,
            token_out,
            amount,
        } => commands::quote::run(config::CHAIN_ID, &token_in, &token_out, amount).await,
        Commands::Pools { limit } => commands::pools::run(limit).await,
    };

    match result {
        Ok(val) => {
            println!("{}", serde_json::to_string_pretty(&val).unwrap());
            std::process::exit(0);
        }
        Err(e) => {
            let err = serde_json::json!({"error": e.to_string()});
            eprintln!("{}", serde_json::to_string_pretty(&err).unwrap());
            std::process::exit(1);
        }
    }
}
