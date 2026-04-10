mod config;
mod calldata;
mod rpc;
mod onchainos;
mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pancakeswap", about = "Swap tokens and manage liquidity on PancakeSwap V3")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a swap quote via QuoterV2 (read-only, no transaction)
    Quote {
        /// Input token address
        #[arg(long)]
        from: String,
        /// Output token address
        #[arg(long)]
        to: String,
        /// Human-readable input amount (e.g. "1.5")
        #[arg(long)]
        amount: String,
        /// Chain ID (56 = BSC, 8453 = Base)
        #[arg(long, default_value = "56")]
        chain: u64,
    },

    /// Swap tokens via SmartRouter exactInputSingle
    Swap {
        /// Input token address
        #[arg(long)]
        from: String,
        /// Output token address
        #[arg(long)]
        to: String,
        /// Human-readable input amount (e.g. "1.5")
        #[arg(long)]
        amount: String,
        /// Slippage tolerance in percent (e.g. 0.5 = 0.5%)
        #[arg(long, default_value = "0.5")]
        slippage: f64,
        /// Chain ID (56 = BSC, 8453 = Base)
        #[arg(long, default_value = "56")]
        chain: u64,
        /// Preview transactions without submitting
        #[arg(long)]
        dry_run: bool,
        /// Confirm broadcast — required to actually submit transactions
        #[arg(long)]
        confirm: bool,
    },

    /// List pools for a token pair via PancakeV3Factory
    Pools {
        /// First token address
        #[arg(long)]
        token0: String,
        /// Second token address
        #[arg(long)]
        token1: String,
        /// Chain ID (56 = BSC, 8453 = Base)
        #[arg(long, default_value = "56")]
        chain: u64,
    },

    /// View LP positions for a wallet address
    Positions {
        /// Wallet address to query
        #[arg(long)]
        owner: String,
        /// Chain ID (56 = BSC, 8453 = Base)
        #[arg(long, default_value = "56")]
        chain: u64,
    },

    /// Add concentrated liquidity via NonfungiblePositionManager.mint
    AddLiquidity {
        /// First token address
        #[arg(long)]
        token_a: String,
        /// Second token address
        #[arg(long)]
        token_b: String,
        /// Fee tier (100, 500, 2500, or 10000)
        #[arg(long, default_value = "500")]
        fee: u32,
        /// Human-readable amount for tokenA
        #[arg(long)]
        amount_a: String,
        /// Human-readable amount for tokenB
        #[arg(long)]
        amount_b: String,
        /// Lower tick boundary (must be multiple of tickSpacing). Omit to auto-compute ±10% range from current pool price.
        #[arg(long, allow_hyphen_values = true)]
        tick_lower: Option<i32>,
        /// Upper tick boundary (must be multiple of tickSpacing). Omit to auto-compute ±10% range from current pool price.
        #[arg(long, allow_hyphen_values = true)]
        tick_upper: Option<i32>,
        /// Slippage tolerance in percent (e.g. 1.0 = 1%)
        #[arg(long, default_value = "1.0")]
        slippage: f64,
        /// Chain ID (56 = BSC, 8453 = Base)
        #[arg(long, default_value = "56")]
        chain: u64,
        /// Preview transactions without submitting
        #[arg(long)]
        dry_run: bool,
        /// Confirm broadcast — required to actually submit transactions
        #[arg(long)]
        confirm: bool,
    },

    /// Remove liquidity from a V3 position (decreaseLiquidity + collect)
    RemoveLiquidity {
        /// NFT position token ID
        #[arg(long)]
        token_id: u128,
        /// Percentage of liquidity to remove (0–100)
        #[arg(long, default_value = "100")]
        liquidity_pct: f64,
        /// Slippage tolerance in percent for minimum amounts out (e.g. 0.5 = 0.5%)
        #[arg(long, default_value = "0.5")]
        slippage: f64,
        /// Chain ID (56 = BSC, 8453 = Base)
        #[arg(long, default_value = "56")]
        chain: u64,
        /// Preview transactions without submitting
        #[arg(long)]
        dry_run: bool,
        /// Confirm broadcast — required to actually submit transactions
        #[arg(long)]
        confirm: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Quote { from, to, amount, chain } => {
            commands::quote::run(commands::quote::QuoteArgs { from, to, amount, chain }).await?;
        }

        Commands::Swap { from, to, amount, slippage, chain, dry_run, confirm } => {
            commands::swap::run(commands::swap::SwapArgs { from, to, amount, slippage, chain, dry_run, confirm }).await?;
        }

        Commands::Pools { token0, token1, chain } => {
            commands::pools::run(commands::pools::PoolsArgs { token0, token1, chain }).await?;
        }

        Commands::Positions { owner, chain } => {
            commands::positions::run(commands::positions::PositionsArgs { owner, chain }).await?;
        }

        Commands::AddLiquidity {
            token_a, token_b, fee, amount_a, amount_b,
            tick_lower, tick_upper, slippage, chain, dry_run, confirm,
        } => {
            commands::add_liquidity::run(commands::add_liquidity::AddLiquidityArgs {
                token_a, token_b, fee, amount_a, amount_b,
                tick_lower, tick_upper, slippage, chain, dry_run, confirm,
            }).await?;
        }

        Commands::RemoveLiquidity { token_id, liquidity_pct, slippage, chain, dry_run, confirm } => {
            commands::remove_liquidity::run(commands::remove_liquidity::RemoveLiquidityArgs {
                token_id, liquidity_pct, slippage, chain, dry_run, confirm,
            }).await?;
        }
    }

    Ok(())
}
