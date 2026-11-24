mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pancakeswap-clmm",
    version,
    about = "PancakeSwap V3 CLMM farming plugin — stake LP NFTs, harvest CAKE, collect fees"
)]
struct Cli {
    /// Chain ID: 56 (BSC), 1 (Ethereum), 8453 (Base), 42161 (Arbitrum). Default: 56 (BSC)
    #[arg(long, default_value = "56")]
    chain: u64,

    /// Simulate without broadcasting (dry-run mode)
    #[arg(long)]
    dry_run: bool,

    /// Confirm and execute the write operation (required for farm, unfarm, harvest, collect-fees)
    #[arg(long)]
    confirm: bool,

    /// Override RPC URL for the selected chain
    #[arg(long)]
    rpc_url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Stake a V3 LP NFT into MasterChefV3 to earn CAKE rewards
    Farm {
        /// LP NFT token ID to stake
        #[arg(long)]
        token_id: u64,
        /// Sender wallet address (defaults to logged-in onchainos wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Withdraw a staked V3 LP NFT from MasterChefV3 (also harvests all pending CAKE)
    Unfarm {
        /// LP NFT token ID to withdraw
        #[arg(long)]
        token_id: u64,
        /// Recipient address for the NFT and harvested CAKE (defaults to logged-in wallet)
        #[arg(long)]
        to: Option<String>,
    },

    /// Claim CAKE rewards for a staked position without withdrawing the NFT
    Harvest {
        /// LP NFT token ID to harvest rewards for
        #[arg(long)]
        token_id: u64,
        /// Recipient address for CAKE (defaults to logged-in wallet)
        #[arg(long)]
        to: Option<String>,
    },

    /// View pending CAKE rewards for a staked LP NFT position
    PendingRewards {
        /// LP NFT token ID
        #[arg(long)]
        token_id: u64,
    },

    /// List active MasterChefV3 farming pools with allocation points and liquidity
    FarmPools,

    /// View all V3 LP positions (unstaked in wallet + optionally staked in MasterChefV3)
    Positions {
        /// Wallet address to query (defaults to logged-in wallet)
        #[arg(long)]
        owner: Option<String>,
        /// Comma-separated list of staked token IDs to include (e.g. "12345,67890")
        #[arg(long)]
        include_staked: Option<String>,
    },

    /// Collect accumulated swap fees from an unstaked V3 LP position
    CollectFees {
        /// LP NFT token ID (must NOT be staked in MasterChefV3)
        #[arg(long)]
        token_id: u64,
        /// Recipient address for fees (defaults to logged-in wallet)
        #[arg(long)]
        recipient: Option<String>,
    },

    /// Check wallet state and get guided next steps
    Quickstart,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Farm { token_id, from } => {
            commands::farm::run(cli.chain, token_id, from, cli.dry_run, cli.confirm, cli.rpc_url).await?;
        }
        Commands::Unfarm { token_id, to } => {
            commands::unfarm::run(cli.chain, token_id, to, cli.dry_run, cli.confirm, cli.rpc_url).await?;
        }
        Commands::Harvest { token_id, to } => {
            commands::harvest::run(cli.chain, token_id, to, cli.dry_run, cli.confirm, cli.rpc_url).await?;
        }
        Commands::PendingRewards { token_id } => {
            commands::pending_rewards::run(cli.chain, token_id, cli.rpc_url).await?;
        }
        Commands::FarmPools => {
            commands::farm_pools::run(cli.chain, cli.rpc_url).await?;
        }
        Commands::Positions {
            owner,
            include_staked,
        } => {
            commands::positions::run(cli.chain, owner, include_staked, cli.rpc_url).await?;
        }
        Commands::CollectFees {
            token_id,
            recipient,
        } => {
            commands::collect_fees::run(cli.chain, token_id, recipient, cli.dry_run, cli.confirm, cli.rpc_url)
                .await?;
        }
        Commands::Quickstart => {
            commands::quickstart::run().await?;
        }
    }

    Ok(())
}
