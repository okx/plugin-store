use clap::{Parser, Subcommand};

mod commands;
mod config;
mod onchainos;
mod rpc;

#[derive(Parser)]
#[command(name = "trader-joe", about = "Trader Joe Liquidity Book DEX — Arbitrum")]
struct Cli {
    /// EVM chain ID (default: 42161 Arbitrum)
    #[arg(long, default_value = "42161")]
    chain: u64,

    /// Simulate without broadcasting (no real transactions)
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get best swap quote from LBQuoter
    Quote {
        /// Input token symbol or address (e.g. USDT, WETH, or 0x...)
        #[arg(long)]
        from: String,

        /// Output token symbol or address
        #[arg(long)]
        to: String,

        /// Human-readable amount to swap (e.g. 0.01)
        #[arg(long)]
        amount: f64,

        /// Decimals of the input token (default: 18; use 6 for USDT/USDC)
        #[arg(long, default_value = "18")]
        decimals: u8,
    },

    /// List all Liquidity Book pools for a token pair
    Pools {
        /// First token symbol or address
        #[arg(long)]
        token_x: String,

        /// Second token symbol or address
        #[arg(long)]
        token_y: String,
    },

    /// Swap tokens on Trader Joe Liquidity Book
    Swap {
        /// Input token symbol or address (e.g. USDT, ETH, WETH)
        #[arg(long)]
        from: String,

        /// Output token symbol or address
        #[arg(long)]
        to: String,

        /// Human-readable amount to swap (e.g. 0.01)
        #[arg(long)]
        amount: f64,

        /// Decimals of the input token (default: 18; use 6 for USDT/USDC)
        #[arg(long, default_value = "18")]
        decimals: u8,

        /// Slippage in basis points (default: 50 = 0.5%)
        #[arg(long, default_value = "50")]
        slippage_bps: u128,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Quote { from, to, amount, decimals } => {
            commands::quote::run(&from, &to, amount, decimals, cli.chain).await
        }
        Commands::Pools { token_x, token_y } => {
            commands::pools::run(&token_x, &token_y, cli.chain).await
        }
        Commands::Swap { from, to, amount, decimals, slippage_bps } => {
            commands::swap::run(
                &from, &to, amount, decimals, slippage_bps, cli.chain, cli.dry_run,
            )
            .await
        }
    };

    if let Err(e) = result {
        let err_out = serde_json::json!({"ok": false, "error": e.to_string()});
        eprintln!("{}", serde_json::to_string_pretty(&err_out).unwrap_or_default());
        std::process::exit(1);
    }
}
