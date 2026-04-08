mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "fluid",
    version = "0.1.0",
    about = "Fluid Protocol — DEX + Lending (fTokens ERC-4626) on Base, Ethereum, and Arbitrum"
)]
struct Cli {
    /// Chain ID: 1 (Ethereum), 8453 (Base), 42161 (Arbitrum)
    #[arg(long, default_value = "8453")]
    chain: u64,

    /// Simulate without broadcasting on-chain
    #[arg(long)]
    dry_run: bool,

    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long)]
    from: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List Fluid fToken lending markets with supply rates
    Markets {
        /// Filter by underlying asset symbol (e.g. USDC, WETH)
        #[arg(long)]
        asset: Option<String>,
    },

    /// View your Fluid lending positions across all fTokens
    Positions,

    /// Supply underlying assets to a Fluid fToken (ERC-4626 deposit)
    Supply {
        /// fToken symbol (fUSDC, fWETH, fGHO, fEURC) or fToken address
        #[arg(long)]
        ftoken: String,

        /// Human-readable amount to supply (e.g. 100 or 0.5)
        #[arg(long)]
        amount: String,
    },

    /// Withdraw underlying assets from a Fluid fToken
    Withdraw {
        /// fToken symbol (fUSDC, fWETH) or fToken address
        #[arg(long)]
        ftoken: String,

        /// Human-readable amount to withdraw (mutually exclusive with --all)
        #[arg(long)]
        amount: Option<String>,

        /// Withdraw entire balance (redeem all shares)
        #[arg(long)]
        all: bool,
    },

    /// Borrow from Fluid Vault (dry-run only — liquidation risk)
    Borrow {
        /// Vault address
        #[arg(long)]
        vault: String,

        /// Human-readable amount to borrow
        #[arg(long)]
        amount: String,
    },

    /// Repay Fluid Vault debt (dry-run only)
    Repay {
        /// Vault address
        #[arg(long)]
        vault: String,

        /// Human-readable amount to repay
        #[arg(long)]
        amount: Option<String>,

        /// Repay entire outstanding balance
        #[arg(long)]
        all: bool,
    },

    /// Swap tokens via Fluid DEX
    Swap {
        /// Input token symbol (EURC, USDC, WETH, WSTETH, WEETH, FLUID, USDE)
        #[arg(long)]
        token_in: String,

        /// Output token symbol
        #[arg(long)]
        token_out: String,

        /// Human-readable input amount (e.g. 100 or 0.001)
        #[arg(long)]
        amount_in: String,

        /// Slippage tolerance in basis points (default: 50 = 0.5%)
        #[arg(long, default_value = "50")]
        slippage_bps: u32,
    },

    /// Get swap quote from Fluid DEX (read-only)
    Quote {
        /// Input token symbol
        #[arg(long)]
        token_in: String,

        /// Output token symbol
        #[arg(long)]
        token_out: String,

        /// Human-readable input amount
        #[arg(long)]
        amount_in: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let chain_id = cli.chain;
    let dry_run = cli.dry_run;
    let from = cli.from.as_deref();

    let result = match cli.command {
        Commands::Markets { asset } => {
            commands::markets::run(chain_id, asset.as_deref()).await
        }
        Commands::Positions => {
            commands::positions::run(chain_id, from).await
        }
        Commands::Supply { ftoken, amount } => {
            commands::supply::run(&ftoken, &amount, chain_id, from, dry_run).await
        }
        Commands::Withdraw { ftoken, amount, all } => {
            commands::withdraw::run(&ftoken, amount.as_deref(), all, chain_id, from, dry_run).await
        }
        Commands::Borrow { vault, amount } => {
            commands::borrow::run(&vault, &amount, chain_id, from, dry_run).await
        }
        Commands::Repay { vault, amount, all } => {
            commands::repay::run(&vault, amount.as_deref(), all, chain_id, from, dry_run).await
        }
        Commands::Swap { token_in, token_out, amount_in, slippage_bps } => {
            commands::swap::run(&token_in, &token_out, &amount_in, slippage_bps, chain_id, from, dry_run).await
        }
        Commands::Quote { token_in, token_out, amount_in } => {
            commands::quote::run(&token_in, &token_out, &amount_in, chain_id).await
        }
    };

    if let Err(e) = result {
        let err_out = serde_json::json!({
            "ok": false,
            "error": e.to_string(),
        });
        eprintln!("{}", serde_json::to_string_pretty(&err_out).unwrap_or_else(|_| e.to_string()));
        std::process::exit(1);
    }
}
