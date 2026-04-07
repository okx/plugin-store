mod abi;
mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "gmx-v1",
    version = "0.1.0",
    about = "Trade perpetuals, swap tokens, and manage GLP liquidity on GMX V1 (Arbitrum/Avalanche)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch current oracle prices for all tokens
    GetPrices {
        /// Chain ID: 42161 (Arbitrum) or 43114 (Avalanche)
        #[arg(long, default_value = "42161")]
        chain: u64,
    },

    /// Fetch open perpetual positions for a wallet
    GetPositions {
        /// Chain ID: 42161 (Arbitrum) or 43114 (Avalanche)
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Wallet address (defaults to logged-in onchainos wallet)
        #[arg(long)]
        account: Option<String>,
    },

    /// Swap tokens via GMX V1 Router (no execution fee required)
    Swap {
        /// Chain ID
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Input token address (ERC-20)
        #[arg(long)]
        input_token: String,
        /// Input amount in token's smallest unit (e.g. 10000000 for 10 USDC)
        #[arg(long)]
        input_amount: u128,
        /// Output token address (ERC-20)
        #[arg(long)]
        output_token: String,
        /// Minimum output amount (0 = no slippage protection)
        #[arg(long, default_value = "0")]
        min_output: u128,
        /// Dry run: print calldata without submitting
        #[arg(long)]
        dry_run: bool,
    },

    /// Buy GLP tokens by depositing ERC-20 tokens (no execution fee required)
    BuyGlp {
        /// Chain ID
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Token to deposit (ERC-20 address)
        #[arg(long)]
        token: String,
        /// Amount to deposit in token's smallest unit
        #[arg(long)]
        amount: u128,
        /// Minimum USDG to receive (0 = no minimum)
        #[arg(long, default_value = "0")]
        min_usdg: u128,
        /// Minimum GLP to receive (0 = no minimum)
        #[arg(long, default_value = "0")]
        min_glp: u128,
        /// Dry run: print calldata without submitting
        #[arg(long)]
        dry_run: bool,
    },

    /// Sell GLP tokens to receive ERC-20 tokens (no execution fee required)
    SellGlp {
        /// Chain ID
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Token to receive (ERC-20 address)
        #[arg(long)]
        token_out: String,
        /// Amount of GLP to redeem (in GLP token units, 18 decimals)
        #[arg(long)]
        glp_amount: u128,
        /// Minimum output tokens to receive (0 = no minimum)
        #[arg(long, default_value = "0")]
        min_out: u128,
        /// Dry run: print calldata without submitting
        #[arg(long)]
        dry_run: bool,
    },

    /// Open a leveraged perpetual position (requires 0.0001 ETH execution fee)
    OpenPosition {
        /// Chain ID
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Collateral token address (e.g. USDC for shorts, WETH for ETH longs)
        #[arg(long)]
        collateral_token: String,
        /// Index token address (the asset to trade, e.g. WETH)
        #[arg(long)]
        index_token: String,
        /// Collateral amount in token's smallest unit
        #[arg(long)]
        amount_in: u128,
        /// Minimum output amount (0 = no slippage protection on collateral swap)
        #[arg(long, default_value = "0")]
        min_out: u128,
        /// Position size in USD (e.g. 1000.0 for $1000)
        #[arg(long)]
        size_usd: f64,
        /// Long position (true) or short (false)
        #[arg(long)]
        is_long: bool,
        /// Acceptable price in GMX 30-decimal format (from get-prices)
        #[arg(long)]
        acceptable_price: u128,
        /// Execution fee override in wei (default: 100000000000000 = 0.0001 ETH)
        #[arg(long)]
        execution_fee: Option<u64>,
        /// Dry run: print calldata without submitting
        #[arg(long)]
        dry_run: bool,
    },

    /// Close a perpetual position (requires 0.0001 ETH execution fee)
    ClosePosition {
        /// Chain ID
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// Collateral token address
        #[arg(long)]
        collateral_token: String,
        /// Index token address (the asset being traded)
        #[arg(long)]
        index_token: String,
        /// Collateral amount to withdraw (0 = leave collateral in position)
        #[arg(long, default_value = "0")]
        collateral_delta: u128,
        /// Position size to close in USD (full position size to close entirely)
        #[arg(long)]
        size_usd: f64,
        /// Long position (true) or short (false)
        #[arg(long)]
        is_long: bool,
        /// Acceptable price in GMX 30-decimal format
        #[arg(long)]
        acceptable_price: u128,
        /// Minimum output tokens (0 = no slippage protection)
        #[arg(long, default_value = "0")]
        min_out: u128,
        /// Execution fee override in wei (default: 100000000000000 = 0.0001 ETH)
        #[arg(long)]
        execution_fee: Option<u64>,
        /// Withdraw as ETH if collateral token is WETH
        #[arg(long, default_value = "false")]
        withdraw_eth: bool,
        /// Dry run: print calldata without submitting
        #[arg(long)]
        dry_run: bool,
    },

    /// Approve an ERC-20 token for GMX V1 Router or GlpManager
    ApproveToken {
        /// Chain ID
        #[arg(long, default_value = "42161")]
        chain: u64,
        /// ERC-20 token address to approve
        #[arg(long)]
        token: String,
        /// Spender address (Router or GlpManager)
        #[arg(long)]
        spender: String,
        /// Dry run: print calldata without submitting
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GetPrices { chain } => {
            commands::get_prices::run(chain).await?;
        }

        Commands::GetPositions { chain, account } => {
            commands::get_positions::run(chain, account).await?;
        }

        Commands::Swap {
            chain,
            input_token,
            input_amount,
            output_token,
            min_output,
            dry_run,
        } => {
            commands::swap::run(chain, &input_token, input_amount, &output_token, min_output, dry_run).await?;
        }

        Commands::BuyGlp {
            chain,
            token,
            amount,
            min_usdg,
            min_glp,
            dry_run,
        } => {
            commands::buy_glp::run(chain, &token, amount, min_usdg, min_glp, dry_run).await?;
        }

        Commands::SellGlp {
            chain,
            token_out,
            glp_amount,
            min_out,
            dry_run,
        } => {
            commands::sell_glp::run(chain, &token_out, glp_amount, min_out, dry_run).await?;
        }

        Commands::OpenPosition {
            chain,
            collateral_token,
            index_token,
            amount_in,
            min_out,
            size_usd,
            is_long,
            acceptable_price,
            execution_fee,
            dry_run,
        } => {
            commands::open_position::run(
                chain,
                &collateral_token,
                &index_token,
                amount_in,
                min_out,
                size_usd,
                is_long,
                acceptable_price,
                execution_fee,
                dry_run,
            )
            .await?;
        }

        Commands::ClosePosition {
            chain,
            collateral_token,
            index_token,
            collateral_delta,
            size_usd,
            is_long,
            acceptable_price,
            min_out,
            execution_fee,
            withdraw_eth,
            dry_run,
        } => {
            commands::close_position::run(
                chain,
                &collateral_token,
                &index_token,
                collateral_delta,
                size_usd,
                is_long,
                acceptable_price,
                min_out,
                execution_fee,
                withdraw_eth,
                dry_run,
            )
            .await?;
        }

        Commands::ApproveToken {
            chain,
            token,
            spender,
            dry_run,
        } => {
            commands::approve_token::run(chain, &token, &spender, dry_run).await?;
        }
    }

    Ok(())
}
