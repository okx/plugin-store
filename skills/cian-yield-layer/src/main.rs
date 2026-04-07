use clap::{Parser, Subcommand};

mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;

#[derive(Parser)]
#[command(
    name = "cian-yield-layer",
    about = "CIAN Yield Layer — ERC4626 vaults on Ethereum (ylstETH, ylpumpBTC)"
)]
struct Cli {
    #[arg(long, global = true, help = "Simulate without broadcasting transactions")]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all vaults with APY and TVL from CIAN REST API
    Vaults,

    /// View your share balance and estimated underlying value in each vault
    Balance {
        /// Wallet address to query (defaults to logged-in wallet)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// View your positions with REST API data (pending redeems, USD value)
    Positions {
        /// Wallet address to query (defaults to logged-in wallet)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Deposit tokens into a vault (approve + optionalDeposit)
    Deposit {
        /// Vault: ylsteth, ylpumpbtc, or vault address
        #[arg(long)]
        vault: String,

        /// Token to deposit: ETH, stETH, wstETH, weETH, eETH, WETH, pumpBTC, WBTC
        #[arg(long)]
        token: String,

        /// Amount to deposit (human-readable, e.g. 1.5 or 1500000000000000000 for raw wei)
        #[arg(long)]
        amount: String,

        /// Sender address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Request async withdrawal — initiates ~5 day redemption process
    RequestRedeem {
        /// Vault: ylsteth, ylpumpbtc, or vault address
        #[arg(long)]
        vault: String,

        /// Number of shares to redeem (e.g. 1.5 or raw uint256)
        #[arg(long)]
        shares: String,

        /// Token to receive: stETH, WETH, wstETH, pumpBTC, WBTC, etc.
        #[arg(long)]
        token: String,

        /// Sender address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Vaults => commands::vaults::run().await,
        Commands::Balance { wallet } => commands::balance::run(wallet.as_deref()).await,
        Commands::Positions { wallet } => commands::positions::run(wallet.as_deref()).await,
        Commands::Deposit { vault, token, amount, from } => {
            commands::deposit::run(&vault, &token, &amount, from.as_deref(), cli.dry_run).await
        }
        Commands::RequestRedeem { vault, shares, token, from } => {
            commands::request_redeem::run(&vault, &shares, &token, from.as_deref(), cli.dry_run).await
        }
    }
}
