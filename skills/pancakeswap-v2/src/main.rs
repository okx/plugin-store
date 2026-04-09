// src/main.rs — PancakeSwap V2 plugin CLI entry point
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use commands::{
    quote, swap, add_liquidity, remove_liquidity, get_pair, get_reserves, lp_balance,
};

#[derive(Parser)]
#[command(name = "pancakeswap-v2", about = "PancakeSwap V2 AMM plugin — swap tokens and manage liquidity on BSC/Base")]
struct Cli {
    /// Chain ID (56 = BSC default, 8453 = Base)
    #[arg(long, default_value = "56")]
    chain: u64,

    /// Slippage tolerance in basis points (default 50 = 0.5%)
    #[arg(long, default_value = "50")]
    slippage_bps: u64,

    /// Swap/LP deadline in seconds from now (default 300 = 5 min)
    #[arg(long, default_value = "300")]
    deadline_secs: u64,

    /// Simulate without broadcasting (no onchainos call made)
    #[arg(long)]
    dry_run: bool,

    /// Override RPC endpoint
    #[arg(long)]
    rpc_url: Option<String>,

    /// Sender address (overrides wallet resolved from onchainos)
    #[arg(long)]
    from: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get expected output for a swap (read-only)
    Quote {
        /// Input token symbol or address (e.g. USDT, CAKE, or 0x...)
        #[arg(long)]
        token_in: String,
        /// Output token symbol or address
        #[arg(long)]
        token_out: String,
        /// Amount of tokenIn in minimal units (e.g. 1000000000000000000 for 1 token with 18 dec)
        #[arg(long)]
        amount_in: u128,
    },

    /// Swap tokens via PancakeSwap V2 Router02
    Swap {
        /// Input token symbol or address
        #[arg(long)]
        token_in: String,
        /// Output token symbol or address
        #[arg(long)]
        token_out: String,
        /// Amount of tokenIn in minimal units
        #[arg(long)]
        amount_in: u128,
    },

    /// Add liquidity to a V2 pair (receive LP tokens)
    AddLiquidity {
        /// First token symbol or address
        #[arg(long)]
        token_a: String,
        /// Second token symbol or address
        #[arg(long)]
        token_b: String,
        /// Desired amount of tokenA in minimal units
        #[arg(long)]
        amount_a: u128,
        /// Desired amount of tokenB (or native BNB/ETH) in minimal units
        #[arg(long)]
        amount_b: u128,
    },

    /// Remove liquidity and withdraw tokens
    RemoveLiquidity {
        /// First token symbol or address
        #[arg(long)]
        token_a: String,
        /// Second token symbol or address
        #[arg(long)]
        token_b: String,
        /// LP token amount to burn (omit to remove all)
        #[arg(long)]
        liquidity: Option<u128>,
    },

    /// Get the pair contract address for two tokens
    GetPair {
        /// First token symbol or address
        #[arg(long)]
        token_a: String,
        /// Second token symbol or address
        #[arg(long)]
        token_b: String,
    },

    /// Get current reserves of a V2 pair
    GetReserves {
        /// First token symbol or address
        #[arg(long)]
        token_a: String,
        /// Second token symbol or address
        #[arg(long)]
        token_b: String,
    },

    /// Get user LP token balance and pool share
    LpBalance {
        /// First token symbol or address
        #[arg(long)]
        token_a: String,
        /// Second token symbol or address
        #[arg(long)]
        token_b: String,
        /// Wallet address to query (defaults to logged-in onchainos wallet)
        #[arg(long)]
        wallet: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Quote { token_in, token_out, amount_in } => {
            quote::run(quote::QuoteArgs {
                chain_id: cli.chain,
                token_in,
                token_out,
                amount_in,
                rpc_url: cli.rpc_url,
            })
            .await
        }

        Commands::Swap { token_in, token_out, amount_in } => {
            swap::run(swap::SwapArgs {
                chain_id: cli.chain,
                token_in,
                token_out,
                amount_in,
                slippage_bps: cli.slippage_bps,
                deadline_secs: cli.deadline_secs,
                from: cli.from,
                rpc_url: cli.rpc_url,
                dry_run: cli.dry_run,
            })
            .await
        }

        Commands::AddLiquidity { token_a, token_b, amount_a, amount_b } => {
            add_liquidity::run(add_liquidity::AddLiquidityArgs {
                chain_id: cli.chain,
                token_a,
                token_b,
                amount_a,
                amount_b,
                slippage_bps: cli.slippage_bps,
                deadline_secs: cli.deadline_secs,
                from: cli.from,
                rpc_url: cli.rpc_url,
                dry_run: cli.dry_run,
            })
            .await
        }

        Commands::RemoveLiquidity { token_a, token_b, liquidity } => {
            remove_liquidity::run(remove_liquidity::RemoveLiquidityArgs {
                chain_id: cli.chain,
                token_a,
                token_b,
                liquidity,
                slippage_bps: cli.slippage_bps,
                deadline_secs: cli.deadline_secs,
                from: cli.from,
                rpc_url: cli.rpc_url,
                dry_run: cli.dry_run,
            })
            .await
        }

        Commands::GetPair { token_a, token_b } => {
            get_pair::run(get_pair::GetPairArgs {
                chain_id: cli.chain,
                token_a,
                token_b,
                rpc_url: cli.rpc_url,
            })
            .await
        }

        Commands::GetReserves { token_a, token_b } => {
            get_reserves::run(get_reserves::GetReservesArgs {
                chain_id: cli.chain,
                token_a,
                token_b,
                rpc_url: cli.rpc_url,
            })
            .await
        }

        Commands::LpBalance { token_a, token_b, wallet } => {
            lp_balance::run(lp_balance::LpBalanceArgs {
                chain_id: cli.chain,
                token_a,
                token_b,
                wallet,
                rpc_url: cli.rpc_url,
            })
            .await
        }
    };

    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default()),
        Err(e) => {
            let err = serde_json::json!({"ok": false, "error": e.to_string()});
            eprintln!("{}", serde_json::to_string_pretty(&err).unwrap_or_default());
            std::process::exit(1);
        }
    }
}
