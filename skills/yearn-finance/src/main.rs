mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "yearn-finance",
    version = "0.1.0",
    about = "Yearn Finance yVault CLI — deposit, withdraw, and track yield on Ethereum"
)]
struct Cli {
    /// Chain ID (default: 1 = Ethereum mainnet)
    #[arg(long, default_value = "1")]
    chain: u64,

    /// Simulate without broadcasting on-chain transactions
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List active Yearn vaults with APR and TVL
    Vaults {
        /// Filter by token symbol (e.g. "USDT", "WETH")
        #[arg(long)]
        token: Option<String>,
    },

    /// Show APR/APY rates for Yearn vaults
    Rates {
        /// Filter by token symbol or vault name
        #[arg(long)]
        token: Option<String>,
    },

    /// Query your positions (shares held) in Yearn vaults
    Positions {
        /// Wallet address to query (default: resolve from onchainos)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Deposit ERC-20 tokens into a Yearn vault
    Deposit {
        /// Vault address or token symbol (e.g. "yvUSDT-1", "USDT", or 0x...)
        #[arg(long)]
        vault: String,

        /// Amount to deposit (e.g. "0.01")
        #[arg(long)]
        amount: String,

        /// Wallet address to use (default: resolve from onchainos)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Withdraw (redeem shares) from a Yearn vault
    Withdraw {
        /// Vault address or token symbol (e.g. "yvUSDT-1", "USDT", or 0x...)
        #[arg(long)]
        vault: String,

        /// Shares to redeem (omit to redeem all)
        #[arg(long)]
        shares: Option<String>,

        /// Wallet address to use (default: resolve from onchainos)
        #[arg(long)]
        wallet: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Vaults { token } => {
            commands::vaults::execute(cli.chain, token.as_deref()).await
        }
        Commands::Rates { token } => {
            commands::rates::execute(cli.chain, token.as_deref()).await
        }
        Commands::Positions { wallet } => {
            commands::positions::execute(cli.chain, wallet.as_deref()).await
        }
        Commands::Deposit { vault, amount, wallet } => {
            commands::deposit::execute(
                cli.chain,
                &vault,
                &amount,
                cli.dry_run,
                wallet.as_deref(),
            ).await
        }
        Commands::Withdraw { vault, shares, wallet } => {
            commands::withdraw::execute(
                cli.chain,
                &vault,
                shares.as_deref(),
                cli.dry_run,
                wallet.as_deref(),
            ).await
        }
    };

    if let Err(e) = result {
        eprintln!("{}", serde_json::json!({
            "ok": false,
            "error": e.to_string()
        }));
        std::process::exit(1);
    }
}
