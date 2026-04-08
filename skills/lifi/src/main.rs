mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lifi", about = "LI.FI/Jumper cross-chain bridge and swap aggregator")]
struct Cli {
    /// Source chain ID (default: 8453 Base)
    #[arg(long, default_value = "8453")]
    chain: u64,

    /// Simulate without broadcasting
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all supported chains
    GetChains,
    /// List tokens on a chain
    GetTokens {
        /// Comma-separated chain IDs (e.g. "1,8453,42161")
        #[arg(long, default_value = "8453")]
        chains: String,
        /// Filter by token symbol (optional)
        #[arg(long)]
        symbol: Option<String>,
    },
    /// Get a bridge/swap quote
    GetQuote {
        /// Source chain ID
        #[arg(long)]
        from_chain: Option<u64>,
        /// Destination chain ID
        #[arg(long)]
        to_chain: u64,
        /// Source token symbol or address
        #[arg(long)]
        from_token: String,
        /// Destination token symbol or address
        #[arg(long)]
        to_token: String,
        /// Amount in token's raw units (e.g., 10000000 for 10 USDT with 6 decimals)
        #[arg(long)]
        amount: String,
        /// Slippage (0.005 = 0.5%)
        #[arg(long, default_value = "0.005")]
        slippage: f64,
        /// Wallet address (resolved from onchainos if not provided)
        #[arg(long)]
        from: Option<String>,
    },
    /// Check cross-chain transfer status
    GetStatus {
        /// Source transaction hash
        #[arg(long)]
        tx_hash: String,
        /// Source chain ID
        #[arg(long)]
        from_chain: Option<u64>,
        /// Destination chain ID
        #[arg(long)]
        to_chain: Option<u64>,
        /// Bridge name (optional)
        #[arg(long)]
        bridge: Option<String>,
    },
    /// List available bridges and DEXes
    GetTools {
        /// Filter by chain ID (optional)
        #[arg(long)]
        chains: Option<String>,
    },
    /// Execute a cross-chain swap or bridge via LI.FI
    Swap {
        /// Source chain ID
        #[arg(long)]
        from_chain: Option<u64>,
        /// Destination chain ID
        #[arg(long)]
        to_chain: u64,
        /// Source token symbol or address
        #[arg(long)]
        from_token: String,
        /// Destination token symbol or address
        #[arg(long)]
        to_token: String,
        /// Amount in token's raw units
        #[arg(long)]
        amount: String,
        /// Slippage (0.005 = 0.5%)
        #[arg(long, default_value = "0.005")]
        slippage: f64,
        /// Wallet address (resolved from onchainos if not provided)
        #[arg(long)]
        from: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let chain_id = cli.chain;
    let dry_run = cli.dry_run;

    let result = match cli.command {
        Commands::GetChains => commands::get_chains::execute().await,
        Commands::GetTokens { chains, symbol } => {
            commands::get_tokens::execute(&chains, symbol.as_deref()).await
        }
        Commands::GetQuote {
            from_chain,
            to_chain,
            from_token,
            to_token,
            amount,
            slippage,
            from,
        } => {
            let src_chain = from_chain.unwrap_or(chain_id);
            commands::get_quote::execute(src_chain, to_chain, &from_token, &to_token, &amount, slippage, from.as_deref()).await
        }
        Commands::GetStatus {
            tx_hash,
            from_chain,
            to_chain,
            bridge,
        } => {
            commands::get_status::execute(&tx_hash, from_chain, to_chain, bridge.as_deref()).await
        }
        Commands::GetTools { chains } => {
            commands::get_tools::execute(chains.as_deref()).await
        }
        Commands::Swap {
            from_chain,
            to_chain,
            from_token,
            to_token,
            amount,
            slippage,
            from,
        } => {
            let src_chain = from_chain.unwrap_or(chain_id);
            commands::swap::execute(src_chain, to_chain, &from_token, &to_token, &amount, slippage, from.as_deref(), dry_run).await
        }
    };

    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
