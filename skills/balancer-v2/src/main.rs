mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "balancer-v2", about = "Balancer V2 DEX Plugin — swap, pool queries, liquidity")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List top Balancer V2 pools on a given chain
    Pools {
        /// Chain ID (default: 42161 Arbitrum)
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Number of pools to return
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Get detailed info for a specific pool
    PoolInfo {
        /// Pool ID (bytes32 pool ID from Balancer)
        #[arg(long)]
        pool: String,
        /// Chain ID (default: 42161 Arbitrum)
        #[arg(long, default_value = "42161")]
        chain: u64,
    },
    /// Get a swap quote via BalancerQueries.querySwap
    Quote {
        /// Input token symbol or address
        #[arg(long)]
        from: String,
        /// Output token symbol or address
        #[arg(long)]
        to: String,
        /// Amount of input token (human-readable, e.g. 0.001)
        #[arg(long)]
        amount: f64,
        /// Pool ID to route through
        #[arg(long)]
        pool: String,
        /// Chain ID (default: 42161 Arbitrum)
        #[arg(long, default_value = "42161")]
        chain: u64,
    },
    /// List LP positions (BPT holdings) for the connected wallet
    Positions {
        /// Chain ID (default: 42161 Arbitrum)
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Wallet address (optional, defaults to connected wallet)
        #[arg(long)]
        wallet: Option<String>,
    },
    /// Execute a token swap via Vault.swap()
    Swap {
        /// Input token symbol or address (e.g. WETH)
        #[arg(long)]
        from: String,
        /// Output token symbol or address (e.g. USDC)
        #[arg(long)]
        to: String,
        /// Amount of input token (human-readable, e.g. 0.001)
        #[arg(long)]
        amount: f64,
        /// Pool ID to swap through
        #[arg(long)]
        pool: String,
        /// Slippage tolerance in % (default: 0.5)
        #[arg(long, default_value = "0.5")]
        slippage: f64,
        /// Chain ID (default: 42161 Arbitrum)
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Simulate without broadcasting
        #[arg(long)]
        dry_run: bool,
    },
    /// Add liquidity to a Balancer V2 pool (joinPool)
    Join {
        /// Pool ID
        #[arg(long)]
        pool: String,
        /// Comma-separated amounts per token (e.g. 0,0,1.0 for 3-token pool)
        #[arg(long)]
        amounts: String,
        /// Chain ID (default: 42161 Arbitrum)
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Simulate without broadcasting
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove liquidity from a Balancer V2 pool (exitPool)
    Exit {
        /// Pool ID
        #[arg(long)]
        pool: String,
        /// BPT amount to burn (human-readable, e.g. 0.001)
        #[arg(long)]
        bpt_amount: f64,
        /// Chain ID (default: 42161 Arbitrum)
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Simulate without broadcasting
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Pools { chain, limit } => {
            commands::pools::run(chain, limit).await
        }
        Commands::PoolInfo { pool, chain } => {
            commands::pool_info::run(&pool, chain).await
        }
        Commands::Quote { from, to, amount, pool, chain } => {
            commands::quote::run(&from, &to, amount, &pool, chain).await
        }
        Commands::Positions { chain, wallet } => {
            commands::positions::run(chain, wallet.as_deref()).await
        }
        Commands::Swap { from, to, amount, pool, slippage, chain, dry_run } => {
            commands::swap::run(&from, &to, amount, &pool, slippage, chain, dry_run).await
        }
        Commands::Join { pool, amounts, chain, dry_run } => {
            let parsed: Result<Vec<f64>, _> = amounts
                .split(',')
                .map(|s| s.trim().parse::<f64>())
                .collect();
            match parsed {
                Ok(amounts_vec) => {
                    commands::join::run(&pool, &amounts_vec, chain, dry_run).await
                }
                Err(e) => Err(anyhow::anyhow!("Failed to parse amounts: {}", e)),
            }
        }
        Commands::Exit { pool, bpt_amount, chain, dry_run } => {
            commands::exit::run(&pool, bpt_amount, chain, dry_run).await
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
