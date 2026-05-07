mod api;
mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "morpho", version, about = "Supply, borrow and earn yield on Morpho — a permissionless lending protocol")]
struct Cli {
    /// Chain ID: 1 (Ethereum) or 8453 (Base) — can also be passed per subcommand
    #[arg(long, default_value = "1", global = true)]
    chain: u64,

    /// Simulate without broadcasting on-chain — can also be passed per subcommand
    #[arg(long, global = true)]
    dry_run: bool,

    /// Confirm and broadcast on-chain (required for write operations; omit to preview)
    #[arg(long, global = true)]
    confirm: bool,

    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long, global = true)]
    from: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Supply assets to a MetaMorpho vault (ERC-4626 deposit)
    Supply {
        /// MetaMorpho vault address
        #[arg(long)]
        vault: String,

        /// Token symbol (USDC, WETH, ...) or ERC-20 address
        #[arg(long)]
        asset: String,

        /// Human-readable amount (e.g. 1000 or 0.5)
        #[arg(long)]
        amount: String,

        /// Chain ID (overrides global --chain)
        #[arg(long)]
        chain: Option<u64>,

        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,

        /// Confirm and broadcast on-chain (overrides global --confirm)
        #[arg(long)]
        confirm: bool,
    },

    /// Withdraw from a MetaMorpho vault (ERC-4626)
    Withdraw {
        /// MetaMorpho vault address
        #[arg(long)]
        vault: String,

        /// Token symbol or ERC-20 address
        #[arg(long)]
        asset: String,

        /// Human-readable amount to withdraw (mutually exclusive with --all)
        #[arg(long)]
        amount: Option<String>,

        /// Withdraw entire balance
        #[arg(long)]
        all: bool,

        /// Chain ID (overrides global --chain)
        #[arg(long)]
        chain: Option<u64>,

        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,

        /// Confirm and broadcast on-chain (overrides global --confirm)
        #[arg(long)]
        confirm: bool,
    },

    /// Borrow from a Morpho Blue market
    Borrow {
        /// Market unique key (bytes32 hex, e.g. 0xabc...)
        #[arg(long)]
        market_id: String,

        /// Human-readable amount to borrow
        #[arg(long)]
        amount: String,

        /// Chain ID (overrides global --chain)
        #[arg(long)]
        chain: Option<u64>,

        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,

        /// Confirm and broadcast on-chain (overrides global --confirm)
        #[arg(long)]
        confirm: bool,
    },

    /// Repay Morpho Blue debt
    Repay {
        /// Market unique key (bytes32 hex)
        #[arg(long)]
        market_id: String,

        /// Human-readable amount to repay (mutually exclusive with --all)
        #[arg(long)]
        amount: Option<String>,

        /// Repay entire outstanding balance
        #[arg(long)]
        all: bool,

        /// Chain ID (overrides global --chain)
        #[arg(long)]
        chain: Option<u64>,

        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,

        /// Confirm and broadcast on-chain (overrides global --confirm)
        #[arg(long)]
        confirm: bool,
    },

    /// View user positions and health factors
    Positions,

    /// List Morpho Blue markets with APYs
    Markets {
        /// Filter by loan asset symbol (e.g. USDC)
        #[arg(long)]
        asset: Option<String>,
    },

    /// Supply collateral to a Morpho Blue market (P1)
    SupplyCollateral {
        /// Market unique key (bytes32 hex)
        #[arg(long)]
        market_id: String,

        /// Human-readable amount of collateral to supply
        #[arg(long)]
        amount: String,

        /// Chain ID (overrides global --chain)
        #[arg(long)]
        chain: Option<u64>,

        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,

        /// Confirm and broadcast on-chain (overrides global --confirm)
        #[arg(long)]
        confirm: bool,
    },

    /// Withdraw collateral from a Morpho Blue market
    WithdrawCollateral {
        /// Market unique key (bytes32 hex)
        #[arg(long)]
        market_id: String,

        /// Human-readable amount of collateral to withdraw (mutually exclusive with --all)
        #[arg(long)]
        amount: Option<String>,

        /// Withdraw all collateral
        #[arg(long)]
        all: bool,

        /// Chain ID (overrides global --chain)
        #[arg(long)]
        chain: Option<u64>,

        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,

        /// Confirm and broadcast on-chain (overrides global --confirm)
        #[arg(long)]
        confirm: bool,
    },

    /// Claim Merkl rewards (P1)
    ClaimRewards {
        /// Chain ID (overrides global --chain)
        #[arg(long)]
        chain: Option<u64>,

        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,

        /// Confirm and broadcast on-chain (overrides global --confirm)
        #[arg(long)]
        confirm: bool,
    },

    /// List MetaMorpho vaults with APYs (P1)
    Vaults {
        /// Filter by asset symbol (e.g. USDC)
        #[arg(long)]
        asset: Option<String>,
    },

    /// Check wallet assets and get a recommended next step for Morpho
    Quickstart,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let global_chain = cli.chain;
    let global_dry_run = cli.dry_run;
    let global_confirm = cli.confirm;
    let from = cli.from.as_deref();

    let result = match cli.command {
        Commands::Supply { vault, asset, amount, chain, dry_run, confirm } => {
            let chain_id = chain.unwrap_or(global_chain);
            let dry_run = dry_run || global_dry_run;
            let confirm = confirm || global_confirm;
            commands::supply::run(&vault, &asset, &amount, chain_id, from, dry_run, confirm).await
        }
        Commands::Withdraw { vault, asset, amount, all, chain, dry_run, confirm } => {
            let chain_id = chain.unwrap_or(global_chain);
            let dry_run = dry_run || global_dry_run;
            let confirm = confirm || global_confirm;
            commands::withdraw::run(&vault, &asset, amount.as_deref(), all, chain_id, from, dry_run, confirm).await
        }
        Commands::Borrow { market_id, amount, chain, dry_run, confirm } => {
            let chain_id = chain.unwrap_or(global_chain);
            let dry_run = dry_run || global_dry_run;
            let confirm = confirm || global_confirm;
            commands::borrow::run(&market_id, &amount, chain_id, from, dry_run, confirm).await
        }
        Commands::Repay { market_id, amount, all, chain, dry_run, confirm } => {
            let chain_id = chain.unwrap_or(global_chain);
            let dry_run = dry_run || global_dry_run;
            let confirm = confirm || global_confirm;
            commands::repay::run(&market_id, amount.as_deref(), all, chain_id, from, dry_run, confirm).await
        }
        Commands::Positions => {
            commands::positions::run(global_chain, from).await
        }
        Commands::Markets { asset } => {
            commands::markets::run(global_chain, asset.as_deref()).await
        }
        Commands::SupplyCollateral { market_id, amount, chain, dry_run, confirm } => {
            let chain_id = chain.unwrap_or(global_chain);
            let dry_run = dry_run || global_dry_run;
            let confirm = confirm || global_confirm;
            commands::supply_collateral::run(&market_id, &amount, chain_id, from, dry_run, confirm).await
        }
        Commands::WithdrawCollateral { market_id, amount, all, chain, dry_run, confirm } => {
            let chain_id = chain.unwrap_or(global_chain);
            let dry_run = dry_run || global_dry_run;
            let confirm = confirm || global_confirm;
            commands::withdraw_collateral::run(&market_id, amount.as_deref(), all, chain_id, from, dry_run, confirm).await
        }
        Commands::ClaimRewards { chain, dry_run, confirm } => {
            let chain_id = chain.unwrap_or(global_chain);
            let dry_run = dry_run || global_dry_run;
            let confirm = confirm || global_confirm;
            commands::claim_rewards::run(chain_id, from, dry_run, confirm).await
        }
        Commands::Vaults { asset } => {
            commands::vaults::run(global_chain, asset.as_deref()).await
        }
        Commands::Quickstart => {
            commands::quickstart::run(global_chain, from).await
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
