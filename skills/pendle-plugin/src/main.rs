mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pendle-plugin",
    about = "Pendle Finance plugin — yield tokenization: buy/sell PT & YT, add/remove liquidity, mint/redeem PT+YT",
    version
)]
struct Cli {
    /// Chain ID (default: 42161 Arbitrum — Pendle's highest TVL chain)
    #[arg(long, default_value = "42161", global = true)]
    chain: u64,

    /// Simulate without broadcasting any transaction
    #[arg(long, global = true)]
    dry_run: bool,

    /// Confirm and broadcast the transaction (required for live execution)
    #[arg(long, global = true)]
    confirm: bool,

    /// Optional Pendle API Bearer token (increases rate limit)
    #[arg(long, global = true)]
    api_key: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Onboarding status — check gas + stable balance + active positions, then suggest the next command.
    Quickstart {
        /// Wallet address to query (defaults to current logged-in wallet)
        #[arg(long)]
        user: Option<String>,
    },

    /// List active Pendle markets with APY and TVL
    ListMarkets {
        /// Filter by chain ID (omit for all chains)
        #[arg(long)]
        chain_id: Option<u64>,

        /// Only show active markets
        #[arg(long)]
        active_only: bool,

        /// Number of results to skip
        #[arg(long, default_value = "0")]
        skip: u64,

        /// Max results to return (max 100)
        #[arg(long, default_value = "20")]
        limit: u64,

        /// Filter markets by name or token symbol (e.g. weETH, USDC, wstETH).
        /// Note: ETH pools use liquid staking derivatives — try weETH, wstETH, rETH instead of ETH/WETH.
        #[arg(long)]
        search: Option<String>,
    },

    /// Get detailed market data for a specific Pendle market
    GetMarket {
        /// Market contract address (also accepted as --market-id)
        #[arg(long, alias = "market-id")]
        market: String,

        /// Time frame for historical data: 1D (1 day), 1W (1 week), 1M (1 month)
        #[arg(long)]
        time_frame: Option<String>,
    },

    /// Get a clean summary of token addresses for a specific Pendle market (PT, YT, SY, LP, underlying)
    GetMarketInfo {
        /// Market contract address (also accepted as --market-id)
        #[arg(long, alias = "market-id")]
        market: String,
    },

    /// Get user positions (PT, YT, LP holdings) across all chains
    GetPositions {
        /// User wallet address (defaults to current logged-in wallet)
        #[arg(long)]
        user: Option<String>,

        /// Filter out positions below this USD value
        #[arg(long)]
        filter_usd: Option<f64>,
    },

    /// Get USD prices for PT/YT/LP/SY assets
    GetAssetPrice {
        /// Comma-separated token addresses
        #[arg(long)]
        ids: Option<String>,

        /// Asset type filter: PT, YT, LP, SY
        #[arg(long)]
        asset_type: Option<String>,

        /// Chain ID to filter by
        #[arg(long)]
        chain_id: Option<u64>,
    },

    /// Buy PT (Principal Token) with underlying token — locks in fixed yield
    BuyPt {
        /// Input token address (underlying asset, e.g. USDC)
        #[arg(long)]
        token_in: String,

        /// Amount to spend in wei (smallest unit)
        #[arg(long)]
        amount_in: String,

        /// PT token address to receive
        #[arg(long)]
        pt_address: String,

        /// Minimum PT amount to receive in wei (slippage protection)
        #[arg(long, default_value = "0")]
        min_pt_out: String,

        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance (0.01 = 1%)
        #[arg(long, default_value = "0.01")]
        slippage: f64,
    },

    /// Sell PT (Principal Token) back to underlying token
    SellPt {
        /// PT token address to sell
        #[arg(long)]
        pt_address: String,

        /// Amount of PT to sell in wei
        #[arg(long)]
        amount_in: String,

        /// Output token address (underlying asset to receive)
        #[arg(long)]
        token_out: String,

        /// Minimum output token amount in wei
        #[arg(long, default_value = "0")]
        min_token_out: String,

        /// Sender wallet address
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance
        #[arg(long, default_value = "0.01")]
        slippage: f64,
    },

    /// Buy YT (Yield Token) — long floating yield position
    BuyYt {
        /// Input token address (underlying asset)
        #[arg(long)]
        token_in: String,

        /// Amount to spend in wei
        #[arg(long)]
        amount_in: String,

        /// YT token address to receive
        #[arg(long)]
        yt_address: String,

        /// Minimum YT amount to receive in wei
        #[arg(long, default_value = "0")]
        min_yt_out: String,

        /// Sender wallet address
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance
        #[arg(long, default_value = "0.01")]
        slippage: f64,
    },

    /// Sell YT (Yield Token) back to underlying token
    SellYt {
        /// YT token address to sell
        #[arg(long)]
        yt_address: String,

        /// Amount of YT to sell in wei
        #[arg(long)]
        amount_in: String,

        /// Output token address
        #[arg(long)]
        token_out: String,

        /// Minimum output token amount in wei
        #[arg(long, default_value = "0")]
        min_token_out: String,

        /// Sender wallet address
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance
        #[arg(long, default_value = "0.01")]
        slippage: f64,
    },

    /// Add single-token liquidity to a Pendle AMM pool
    AddLiquidity {
        /// Input token address
        #[arg(long)]
        token_in: String,

        /// Amount to deposit in wei
        #[arg(long)]
        amount_in: String,

        /// LP token address of the target pool
        #[arg(long)]
        lp_address: String,

        /// Minimum LP tokens to receive in wei
        #[arg(long, default_value = "0")]
        min_lp_out: String,

        /// Sender wallet address
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance
        #[arg(long, default_value = "0.005")]
        slippage: f64,
    },

    /// Remove single-token liquidity from a Pendle AMM pool
    RemoveLiquidity {
        /// LP token address of the pool
        #[arg(long)]
        lp_address: String,

        /// Amount of LP tokens to burn in wei
        #[arg(long)]
        lp_amount_in: String,

        /// Output token address (underlying asset to receive)
        #[arg(long)]
        token_out: String,

        /// Minimum output token amount in wei
        #[arg(long, default_value = "0")]
        min_token_out: String,

        /// Sender wallet address
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance
        #[arg(long, default_value = "0.005")]
        slippage: f64,
    },

    /// Mint PT + YT pair from underlying token
    MintPy {
        /// Input token address (underlying asset)
        #[arg(long)]
        token_in: String,

        /// Amount to mint from in wei
        #[arg(long)]
        amount_in: String,

        /// PT token address
        #[arg(long)]
        pt_address: String,

        /// YT token address
        #[arg(long)]
        yt_address: String,

        /// Sender wallet address
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance
        #[arg(long, default_value = "0.005")]
        slippage: f64,
    },

    /// Redeem equal amounts of PT + YT back to underlying token
    RedeemPy {
        /// PT token address
        #[arg(long)]
        pt_address: String,

        /// Amount of PT to redeem in wei
        #[arg(long)]
        pt_amount: String,

        /// YT token address
        #[arg(long)]
        yt_address: String,

        /// Amount of YT to redeem in wei (must equal PT amount)
        #[arg(long)]
        yt_amount: String,

        /// Output token address (underlying asset to receive)
        #[arg(long)]
        token_out: String,

        /// Sender wallet address
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance
        #[arg(long, default_value = "0.005")]
        slippage: f64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let chain = cli.chain;
    let dry_run = cli.dry_run;
    let api_key = cli.api_key.as_deref();

    let confirm = cli.confirm;

    let result = match cli.command {
        Commands::Quickstart { user } => {
            commands::quickstart::run(user.as_deref(), chain, api_key).await
        }

        Commands::ListMarkets {
            chain_id,
            active_only,
            skip,
            limit,
            search,
        } => {
            // If --chain-id not explicitly passed, default to the global --chain value
            // so `pendle --chain 42161 list-markets` correctly filters by Arbitrum.
            let effective_chain_id = Some(chain_id.unwrap_or(chain));
            commands::list_markets::run(
                effective_chain_id,
                if active_only { Some(true) } else { None },
                skip,
                limit,
                search.as_deref(),
                api_key,
            )
            .await
        }

        Commands::GetMarket { market, time_frame } => {
            commands::get_market::run(chain, &market, time_frame.as_deref(), api_key).await
        }

        Commands::GetMarketInfo { market } => {
            commands::get_market_info::run(chain, &market, api_key).await
        }

        Commands::GetPositions { user, filter_usd } => {
            commands::get_positions::run(user.as_deref(), chain, filter_usd, api_key).await
        }

        Commands::GetAssetPrice {
            ids,
            asset_type,
            chain_id,
        } => {
            commands::get_asset_price::run(chain_id, ids.as_deref(), asset_type.as_deref(), api_key)
                .await
        }

        Commands::BuyPt {
            token_in,
            amount_in,
            pt_address,
            min_pt_out,
            from,
            slippage,
        } => {
            commands::buy_pt::run(
                chain,
                &token_in,
                &amount_in,
                &pt_address,
                &min_pt_out,
                from.as_deref(),
                slippage,
                dry_run,
                confirm,
                api_key,
            )
            .await
        }

        Commands::SellPt {
            pt_address,
            amount_in,
            token_out,
            min_token_out,
            from,
            slippage,
        } => {
            commands::sell_pt::run(
                chain,
                &pt_address,
                &amount_in,
                &token_out,
                &min_token_out,
                from.as_deref(),
                slippage,
                dry_run,
                confirm,
                api_key,
            )
            .await
        }

        Commands::BuyYt {
            token_in,
            amount_in,
            yt_address,
            min_yt_out,
            from,
            slippage,
        } => {
            commands::buy_yt::run(
                chain,
                &token_in,
                &amount_in,
                &yt_address,
                &min_yt_out,
                from.as_deref(),
                slippage,
                dry_run,
                confirm,
                api_key,
            )
            .await
        }

        Commands::SellYt {
            yt_address,
            amount_in,
            token_out,
            min_token_out,
            from,
            slippage,
        } => {
            commands::sell_yt::run(
                chain,
                &yt_address,
                &amount_in,
                &token_out,
                &min_token_out,
                from.as_deref(),
                slippage,
                dry_run,
                confirm,
                api_key,
            )
            .await
        }

        Commands::AddLiquidity {
            token_in,
            amount_in,
            lp_address,
            min_lp_out,
            from,
            slippage,
        } => {
            commands::add_liquidity::run(
                chain,
                &token_in,
                &amount_in,
                &lp_address,
                &min_lp_out,
                from.as_deref(),
                slippage,
                dry_run,
                confirm,
                api_key,
            )
            .await
        }

        Commands::RemoveLiquidity {
            lp_address,
            lp_amount_in,
            token_out,
            min_token_out,
            from,
            slippage,
        } => {
            commands::remove_liquidity::run(
                chain,
                &lp_address,
                &lp_amount_in,
                &token_out,
                &min_token_out,
                from.as_deref(),
                slippage,
                dry_run,
                confirm,
                api_key,
            )
            .await
        }

        Commands::MintPy {
            token_in,
            amount_in,
            pt_address,
            yt_address,
            from,
            slippage,
        } => {
            commands::mint_py::run(
                chain,
                &token_in,
                &amount_in,
                &pt_address,
                &yt_address,
                from.as_deref(),
                slippage,
                dry_run,
                confirm,
                api_key,
            )
            .await
        }

        Commands::RedeemPy {
            pt_address,
            pt_amount,
            yt_address,
            yt_amount,
            token_out,
            from,
            slippage,
        } => {
            commands::redeem_py::run(
                chain,
                &pt_address,
                &pt_amount,
                &yt_address,
                &yt_amount,
                &token_out,
                from.as_deref(),
                slippage,
                dry_run,
                confirm,
                api_key,
            )
            .await
        }
    };

    match result {
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap_or_default());
        }
        Err(e) => {
            let error_output = serde_json::json!({
                "ok": false,
                "error": e.to_string()
            });
            eprintln!("{}", serde_json::to_string_pretty(&error_output).unwrap_or_default());
            std::process::exit(1);
        }
    }
}
