// src/main.rs — Clanker plugin CLI entry point
mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "clanker", about = "Clanker token launch plugin for OnchainOS", version)]
struct Cli {
    /// Chain ID (default: 8453 Base; also supports 42161 Arbitrum One)
    #[arg(long, default_value = "8453")]
    chain: u64,

    /// Simulate without broadcasting (skips on-chain calls)
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List recently deployed Clanker tokens
    ListTokens {
        /// Page number (1-based)
        #[arg(long, default_value = "1")]
        page: u32,

        /// Number of tokens per page (max 50)
        #[arg(long, default_value = "20")]
        limit: u32,

        /// Sort direction: asc or desc
        #[arg(long, default_value = "desc")]
        sort: String,
    },

    /// Search tokens by creator wallet address or Farcaster username
    SearchTokens {
        /// Wallet address (0x...) or Farcaster username
        #[arg(long)]
        query: String,

        /// Max number of results (max 50)
        #[arg(long, default_value = "20")]
        limit: u32,

        /// Pagination offset
        #[arg(long, default_value = "0")]
        offset: u32,

        /// Sort direction: asc or desc
        #[arg(long, default_value = "desc")]
        sort: String,

        /// Only return tokens from trusted deployers
        #[arg(long)]
        trusted_only: bool,
    },

    /// Query on-chain info and price for a Clanker token
    TokenInfo {
        /// Token contract address
        #[arg(long)]
        address: String,
    },

    /// Deploy a new ERC-20 token directly on-chain via the Clanker V4 factory (no API key required)
    DeployToken {
        /// Token name (e.g. "SkyDog")
        #[arg(long)]
        name: String,

        /// Token symbol (e.g. "SKYDOG")
        #[arg(long)]
        symbol: String,

        /// Deployer wallet address (defaults to logged-in onchainos wallet)
        #[arg(long)]
        from: Option<String>,

        /// Token image URL (IPFS or HTTPS)
        #[arg(long)]
        image_url: Option<String>,

        /// Confirm and execute the deployment (required after reviewing --dry-run output)
        #[arg(long)]
        confirm: bool,
    },

    /// Claim LP fee rewards for a Clanker token you created
    ClaimRewards {
        /// Token contract address to claim rewards for
        #[arg(long)]
        token_address: String,

        /// Wallet address to receive rewards (defaults to logged-in onchainos wallet)
        #[arg(long)]
        from: Option<String>,

        /// Confirm and execute the claim (required after reviewing --dry-run output)
        #[arg(long)]
        confirm: bool,
    },

    /// Check wallet state and get personalised onboarding steps
    Quickstart,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::ListTokens { page, limit, sort } => {
            commands::list_tokens::run(page, limit, &sort, Some(cli.chain)).await
        }

        Commands::SearchTokens {
            query,
            limit,
            offset,
            sort,
            trusted_only,
        } => commands::search_tokens::run(&query, limit, offset, &sort, trusted_only).await,

        Commands::TokenInfo { address } => {
            commands::token_info::run(cli.chain, &address)
                .map_err(|e| anyhow::anyhow!(e))
        }

        Commands::DeployToken {
            name,
            symbol,
            from,
            image_url,
            confirm,
        } => {
            commands::deploy_token::run(
                cli.chain,
                &name,
                &symbol,
                from.as_deref(),
                image_url.as_deref(),
                cli.dry_run,
                confirm,
            )
            .await
        }

        Commands::ClaimRewards {
            token_address,
            from,
            confirm,
        } => {
            commands::claim_rewards::run(
                cli.chain,
                &token_address,
                from.as_deref(),
                cli.dry_run,
                confirm,
            )
            .await
        }

        Commands::Quickstart => {
            match commands::quickstart::run(cli.chain).await {
                Ok(val) => {
                    println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    };

    if let Err(e) = result {
        let error_output = serde_json::json!({
            "ok": false,
            "error": e.to_string()
        });
        eprintln!("{}", serde_json::to_string_pretty(&error_output).unwrap_or_default());
        std::process::exit(1);
    }
}
