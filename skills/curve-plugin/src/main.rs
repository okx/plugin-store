mod api;
mod commands;
mod config;
mod curve_abi;
mod multicall;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "curve", version, about = "Curve DEX plugin — swap, add/remove liquidity, query pools")]
struct Cli {
    /// Chain ID (1=Ethereum, 42161=Arbitrum, 8453=Base, 137=Polygon, 56=BSC)
    #[arg(long, default_value = "1")]
    chain: u64,

    /// Simulate without broadcasting on-chain transactions
    #[arg(long)]
    dry_run: bool,

    /// Execute write operations on-chain (swap, add-liquidity, remove-liquidity). Without this flag write operations show a preview and exit. Has no effect on read-only commands.
    #[arg(long)]
    confirm: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List Curve pools on the specified chain
    GetPools {
        /// Registry to query: main, crypto, factory, factory-crypto (omit for all)
        #[arg(long)]
        registry: Option<String>,

        /// Maximum number of pools to display (sorted by TVL)
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Get detailed info for a specific Curve pool
    GetPoolInfo {
        /// Pool contract address
        #[arg(long)]
        pool: String,
    },

    /// Query user LP token balances across Curve pools
    GetBalances {
        /// Wallet address to query (default: onchainos active wallet)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Get a swap quote from CurveRouterNG (read-only)
    Quote {
        /// Input token symbol or address
        #[arg(long)]
        token_in: String,

        /// Output token symbol or address
        #[arg(long)]
        token_out: String,

        /// Input amount in human-readable units (e.g. 1.0 = 1 USDC)
        #[arg(long)]
        amount: f64,

        /// Slippage tolerance (default: 0.005 = 0.5%)
        #[arg(long, default_value = "0.005")]
        slippage: f64,
    },

    /// Execute a token swap via CurveRouterNG
    Swap {
        /// Input token symbol or address
        #[arg(long)]
        token_in: String,

        /// Output token symbol or address
        #[arg(long)]
        token_out: String,

        /// Input amount in human-readable units (e.g. 1.0 = 1 USDC)
        #[arg(long)]
        amount: f64,

        /// Slippage tolerance (default: 0.005 = 0.5%)
        #[arg(long, default_value = "0.005")]
        slippage: f64,

        /// Sender wallet address (default: onchainos active wallet)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Add liquidity to a Curve pool
    AddLiquidity {
        /// Pool contract address
        #[arg(long)]
        pool: String,

        /// Comma-separated token amounts in human-readable units matching pool coin order (e.g. "0,500.0,500.0" for 3pool: DAI,USDC,USDT)
        #[arg(long)]
        amounts: String,

        /// Minimum LP tokens to accept in human-readable units (default: 0)
        #[arg(long, default_value = "0")]
        min_mint: String,

        /// Sender wallet address (default: onchainos active wallet)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Remove liquidity from a Curve pool
    RemoveLiquidity {
        /// Pool contract address
        #[arg(long)]
        pool: String,

        /// LP token amount to redeem in human-readable units (default: full balance)
        #[arg(long)]
        lp_amount: Option<String>,

        /// Coin index for single-coin withdrawal (omit for proportional)
        #[arg(long, allow_hyphen_values = true)]
        coin_index: Option<i64>,

        /// Comma-separated minimum output amounts in human-readable units (default: "0" or "0,0" etc.)
        #[arg(long, default_value = "0")]
        min_amounts: String,

        /// Sender wallet address (default: onchainos active wallet)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Check wallet state and get personalised onboarding steps
    Quickstart {
        /// Wallet address to check (default: onchainos active wallet)
        #[arg(long)]
        wallet: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let chain_id = cli.chain;
    const SUPPORTED_CHAINS: &[u64] = &[1, 42161, 8453, 137, 56];
    if !SUPPORTED_CHAINS.contains(&chain_id) {
        eprintln!(
            "{}",
            serde_json::json!({"ok": false, "error": format!(
                "Chain {} is not supported. Supported chains: 1 (Ethereum), 42161 (Arbitrum), 8453 (Base), 137 (Polygon), 56 (BSC)",
                chain_id
            )})
        );
        std::process::exit(1);
    }
    let dry_run = cli.dry_run;

    let result = match cli.command {
        Commands::GetPools { registry, limit } => {
            commands::get_pools::run(chain_id, registry, limit).await
        }
        Commands::GetPoolInfo { pool } => {
            commands::get_pool_info::run(chain_id, pool).await
        }
        Commands::GetBalances { wallet } => {
            commands::get_balances::run(chain_id, wallet).await
        }
        Commands::Quote {
            token_in,
            token_out,
            amount,
            slippage,
        } => {
            commands::quote::run(chain_id, token_in, token_out, amount, slippage).await
        }
        Commands::Swap {
            token_in,
            token_out,
            amount,
            slippage,
            wallet,
        } => {
            commands::swap::run(chain_id, token_in, token_out, amount, slippage, wallet, dry_run, cli.confirm)
                .await
        }
        Commands::AddLiquidity {
            pool,
            amounts,
            min_mint,
            wallet,
        } => {
            let amount_strs: Vec<String> = amounts
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            commands::add_liquidity::run(chain_id, pool, amount_strs, min_mint, wallet, dry_run, cli.confirm)
                .await
        }
        Commands::RemoveLiquidity {
            pool,
            lp_amount,
            coin_index,
            min_amounts,
            wallet,
        } => {
            let min_amount_strs: Vec<String> = min_amounts
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            commands::remove_liquidity::run(
                chain_id,
                pool,
                lp_amount,
                coin_index,
                min_amount_strs,
                wallet,
                dry_run,
                cli.confirm,
            )
            .await
        }

        Commands::Quickstart { wallet } => {
            match commands::quickstart::run(chain_id, wallet.as_deref()).await {
                Ok(val) => {
                    println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    };

    if let Err(e) = result {
        eprintln!(
            "{}",
            serde_json::json!({ "ok": false, "error": e.to_string() })
        );
        std::process::exit(1);
    }
}
